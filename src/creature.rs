use std::{collections::HashSet, time::Duration};

use avian2d::spatial_query::{SpatialQuery, SpatialQueryFilter};
use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;

use crate::{
    body::{ConnectedBodies, Locomotor},
    brain::{Action, Belief, Brain, Sense, SmartEntity},
    environment::{AuditoryEventType, EventType, InternalEventType, SenseEvent, VisualEventType},
    food::Food,
    pathfind::Pathfinding,
    tilemap::{CurrentTile, PhysicsTilemap},
};

#[derive(Component)]
pub struct Prey {
    pub pos: Vec2,
    hunger: f32,
    movement: Vec2,
    eating: bool,
    walk_timer: Timer,
}

// pub fn setup(commands: &mut Commands, pos: Vec2) {
//     commands.spawn((
//         Prey {
//             pos,
//             hunger: 0.7,
//             movement: Vec2::ZERO,
//             eating: false,
//             walk_timer: Timer::new(Duration::from_millis(400), TimerMode::Repeating),
//         },
//         Brain::new(vec![
//             Box::new(RunState::new()),
//             Box::new(HungerState::new()),
//         ]),
//         Sprite::from_color(Color::WHITE, Vec2::new(20.0, 20.0)),
//         Transform::from_xyz(pos.x - 10.0, pos.y - 10.0, 0.0),
//         Visibility::default(),
//     ));
// }

pub fn eat(
    mut commands: Commands,
    creature_query: Query<&mut Prey>,
    food_query: Query<(Entity, &Food)>,
) {
    for mut creature in creature_query {
        if !creature.eating {
            continue;
        }

        for (food_entity, food) in food_query {
            if creature.pos.distance(food.pos) < 10.0 {
                creature.hunger = 0.0;
                commands.entity(food_entity).despawn();
            }
        }

        creature.eating = false;
    }
}

pub fn process(
    mut commands: Commands,
    time: Res<Time>,
    events_query: Query<&SenseEvent>,
    creature_query: Query<(
        &mut Prey,
        &mut Brain<SenseEvent, Action<CreatureAction>>,
        &mut Transform,
        &CurrentTile,
    )>,
    physics_tilemap: Res<PhysicsTilemap>,
    q_tilemap: Query<(
        &TilemapSize,
        &TilemapGridSize,
        &TilemapTileSize,
        &TilemapType,
        &TilemapAnchor,
    )>,
) {
    for (mut creature, mut brain, mut transform, current) in creature_query {
        creature.walk_timer.tick(time.delta());

        let Some(current) = current.tile else {
            continue;
        };
        let mut tiles = HashSet::new();
        tiles.insert(current);

        if creature.walk_timer.just_finished() {
            commands.spawn(SenseEvent::new(
                EventType::Auditory {
                    typ: AuditoryEventType::Scuttle,
                    affects: tiles,
                },
                creature.pos,
                0.6,
                5.0,
            ));
        }

        let mut senses = Vec::new();
        for event in events_query {
            match &event.event_type {
                EventType::Auditory { affects, .. } => {
                    if affects.contains(&current) {
                        senses.push(Sense::new(
                            (event.position - creature.pos).to_angle(),
                            creature.pos.distance(event.position),
                            0.6,
                            0.6,
                            event.clone(),
                        ));
                    }
                }
                EventType::Visual(_) => {
                    senses.push(Sense::new(
                        (event.position - creature.pos).to_angle(),
                        creature.pos.distance(event.position),
                        0.95,
                        0.8,
                        event.clone(),
                    ));
                }
                _ => (),
            }
        }

        let (map_size, grid_size, tile_size, map_type, anchor) =
            q_tilemap.get(physics_tilemap.0).unwrap();

        creature.hunger += 0.1 * time.delta_secs();
        creature.hunger = creature.hunger.min(1.0);
        senses.push(Sense::internal(SenseEvent::internal(EventType::Internal(
            InternalEventType::Hunger {
                intensity: creature.hunger,
            },
        ))));
        senses.push(Sense::internal(SenseEvent::internal(EventType::Internal(
            InternalEventType::Movement {
                amount: creature.movement * time.delta_secs(),
            },
        ))));

        brain.apply(&mut *creature, senses, time.delta_secs());
    }
}

pub fn translate(
    mut gizmos: Gizmos,
    spatial_query: SpatialQuery,
    query: Query<(
        Entity,
        &mut Locomotor,
        &mut Pathfinding,
        &Prey,
        &Transform,
        Option<&ConnectedBodies>,
    )>,
) {
    for (entity, mut locomotor, mut pathfinding, prey, transform, connected) in query {
        const DIRECTIONS: [Dir2; 4] = [Dir2::NEG_X, Dir2::X, Dir2::NEG_Y, Dir2::Y];

        let pos = transform.translation.xy();

        pathfinding.from = pos;
        pathfinding.to = pos + prey.movement;
        let Some(direction) = pathfinding.direction else {
            continue;
        };
        let mut movement = direction * prey.movement.length();

        let mut excluded = vec![entity];
        if let Some(connected) = connected {
            for connected in &connected.0 {
                excluded.push(*connected);
            }
        }

        for direction in DIRECTIONS {
            if let Some(_data) = spatial_query.cast_ray(
                pos,
                direction,
                20.0,
                true,
                &SpatialQueryFilter::from_excluded_entities(excluded.clone()),
            ) {
                if direction.x.abs() > 0.5 && movement.y > 100.0 {
                    movement.x = direction.x * 50.0;
                }
                if direction.y.abs() > 0.5 && direction.y.signum() == movement.y.signum() {
                    movement.y = 0.0;
                }
                gizmos.line_2d(pos, pos + direction.as_vec2() * 20.0, Color::BLACK);
            }
        }

        locomotor.desired_velocity = movement;
    }
}

impl SmartEntity<Action<CreatureAction>> for Prey {
    fn apply(&mut self, commands: Vec<Action<CreatureAction>>, delta: f32) {
        let max = commands
            .iter()
            .max_by(|left, right| left.weight.abs().total_cmp(&right.weight.abs()))
            .map(|command| command.weight)
            .unwrap_or(1.0);
        let commands = commands
            .iter()
            .map(|command| Action {
                command: command.command,
                weight: command.weight / max,
            })
            .collect::<Vec<_>>();
        let magnitude = commands
            .iter()
            .map(|command| match command.command {
                CreatureAction::Movement(movement) => movement.length(),
                _ => 0.0,
            })
            .max_by(f32::total_cmp)
            .unwrap_or(0.0);
        let mut movement = Vec2::new(0.01, 0.01);
        for command in commands {
            match command.command {
                CreatureAction::Movement(amount) => movement += amount * command.weight,
                CreatureAction::Eat => self.eating = true,
            }
        }
        self.movement = self.movement.lerp(movement.normalize() * magnitude, 0.5);
    }
}

#[derive(Debug, Clone, Copy)]
pub enum CreatureAction {
    Movement(Vec2),
    Eat,
}

struct RunState {
    rng: rand::rngs::StdRng,
    position: Option<Vec2>,
    urgentness: f32,
}

impl RunState {
    const MAX_DISTANCE: f32 = 500.0;
    const MAX_SPEED: f32 = 250.0;

    pub fn new() -> Self {
        Self {
            rng: rand::make_rng(),
            position: None,
            urgentness: 0.0,
        }
    }
}

impl Belief<SenseEvent, Action<CreatureAction>> for RunState {
    fn decay(&mut self, delta: f32) {
        self.urgentness -= 0.2 * delta;

        if self.urgentness <= 0.0 {
            self.urgentness = 0.0;
            self.position = None;
        }
    }

    fn update(&mut self, sense: Sense<SenseEvent>) {
        match sense.sense_type.event_type {
            EventType::Auditory {
                typ: AuditoryEventType::Walk,
                ..
            } => {
                let (distance, _, vector) = sense.get_pos(&mut self.rng);
                if distance > Self::MAX_DISTANCE {
                    return;
                }

                // More dynamic urgentness so the creature gets more scared
                // when it hears more disturbing things.
                self.urgentness = self
                    .urgentness
                    .lerp(
                        (1.0 - distance / Self::MAX_DISTANCE) * 0.15 + self.urgentness * 0.9,
                        0.1,
                    )
                    .min(2.0);

                match &mut self.position {
                    Some(position) => *position = position.lerp(vector, 0.05),
                    None => self.position = Some(vector),
                }
            }
            _ => (),
        }
    }

    fn minimize(&mut self) -> Vec<Action<CreatureAction>> {
        if let Some(pos) = self.position {
            vec![Action {
                command: CreatureAction::Movement(
                    -pos.normalize() * self.urgentness * Self::MAX_SPEED,
                ),
                weight: self.urgentness * 10.0,
            }]
        } else {
            Vec::new()
        }
    }
}

struct HungerState {
    rng: rand::rngs::StdRng,
    food_offset: Option<Vec2>,
    this_food_offset: Option<Vec2>,
    precision: f32,
    hunger: f32,
}

impl HungerState {
    const MAX_DISTANCE: f32 = 1000.0;
    const MAX_SPEED: f32 = 220.0;

    pub fn new() -> Self {
        Self {
            rng: rand::make_rng(),
            food_offset: None,
            this_food_offset: None,
            precision: 0.0,
            hunger: 0.0,
        }
    }
}

impl Belief<SenseEvent, Action<CreatureAction>> for HungerState {
    fn decay(&mut self, delta: f32) {
        self.precision -= 0.05 * delta;

        if self.precision <= 0.0 {
            self.precision = 0.0;
            self.food_offset = None;
        }
    }

    fn update(&mut self, sense: Sense<SenseEvent>) {
        match sense.sense_type.event_type {
            EventType::Visual(VisualEventType::Food { .. }) => {
                let (distance, _, vector) = sense.get_pos(&mut self.rng);
                if distance > Self::MAX_DISTANCE {
                    return;
                }

                self.precision = sense.distance_precision * sense.direction_precision;

                match &mut self.this_food_offset {
                    Some(pos) => {
                        if vector.length() < pos.length() {
                            *pos = pos.lerp(vector, 0.2)
                        }
                    }
                    None => self.this_food_offset = Some(vector),
                }
            }
            EventType::Internal(InternalEventType::Hunger { intensity }) => {
                self.hunger = intensity;
            }
            EventType::Internal(InternalEventType::Movement { amount }) => {
                if let Some(offset) = &mut self.food_offset {
                    *offset -= amount;
                }
            }
            _ => (),
        }
    }

    fn minimize(&mut self) -> Vec<Action<CreatureAction>> {
        if let Some(this_offset) = &self.this_food_offset {
            match &mut self.food_offset {
                Some(offset) => {
                    *offset = offset.lerp(*this_offset, 0.5);
                }
                None => self.food_offset = Some(*this_offset),
            }
        }
        self.this_food_offset = None;

        if self.hunger > 0.8
            && let Some(offset) = self.food_offset
        {
            let mut out = vec![Action {
                command: CreatureAction::Movement(
                    offset.normalize() * self.precision * Self::MAX_SPEED,
                ),
                weight: (self.hunger - 0.8) * 5.0 * 20.0,
            }];

            if offset.length() < 10.0 {
                out.push(Action {
                    command: CreatureAction::Eat,
                    weight: 20.0,
                });
                self.food_offset = None; // prevent phantom food in the future
            }

            out
        } else if let Some(offset) = self.food_offset {
            vec![Action {
                command: CreatureAction::Movement(
                    offset.normalize() * self.precision * Self::MAX_SPEED,
                ),
                weight: 5.0,
            }]
        } else {
            Vec::new()
        }
    }
}
