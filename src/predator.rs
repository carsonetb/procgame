use std::{collections::HashSet, time::Duration};

use avian2d::prelude::*;
use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;
use rand::RngExt;

use crate::{
    body::*,
    brain::*,
    creature::Prey,
    environment::*,
    pathfind::Pathfinding,
    tilemap::{CurrentTile, PhysicsTilemap},
};

#[derive(Debug, Clone, Copy)]
pub enum PredatorAction {
    Movement(Vec2),
    Attack,
}

#[derive(Component)]
pub struct Predator {
    desired_movement: Vec2,
    hunger: f32,
    attacking: bool,
    walk_timer: Timer,
}

impl Predator {
    pub fn new() -> Self {
        Self {
            desired_movement: Vec2::ZERO,
            hunger: 0.0,
            attacking: false,
            walk_timer: Timer::from_seconds(0.1, TimerMode::Repeating),
        }
    }

    pub fn brain() -> Brain<SenseEvent, Action<PredatorAction>> {
        Brain::new(vec![
            Box::new(AttackState::new()),
            Box::new(NoRepeatState::new()),
        ])
    }
}

pub fn attack(
    mut commands: Commands,
    predator_query: Query<(&mut Predator, &Transform)>,
    prey_query: Query<(Entity, &Prey)>,
) {
    for (mut predator, transform) in predator_query {
        if !predator.attacking {
            continue;
        }

        let pos = transform.translation.xy();
        for (prey_entity, prey) in prey_query {
            if pos.distance(prey.pos) < 20.0 {
                predator.hunger = 0.0;
                commands.entity(prey_entity).despawn(); // TODO: Spawn dead version which can be carried.
            }
        }
    }
}

pub fn process(
    mut commands: Commands,
    time: Res<Time>,
    events_query: Query<&SenseEvent>,
    predator_query: Query<(
        Forces,
        &mut Predator,
        &mut Brain<SenseEvent, Action<PredatorAction>>,
        &Transform,
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
    // TODO: Unify a lot of this logic into a helper.
    for (forces, mut predator, mut brain, transform, current) in predator_query {
        let pos = transform.translation.xy();
        predator.walk_timer.tick(time.delta());

        let Some(current) = current.tile else {
            continue;
        };
        let mut tiles = HashSet::new();
        tiles.insert(current);

        if predator.walk_timer.just_finished() {
            commands.spawn(SenseEvent::new(
                EventType::Auditory {
                    typ: AuditoryEventType::Walk,
                    affects: tiles,
                },
                pos,
                0.7,
                5.0,
            ));
        }

        let mut senses = Vec::new();
        for event in events_query {
            match &event.event_type {
                EventType::Auditory { affects, .. } => {
                    if affects.contains(&current) {
                        senses.push(Sense::new(
                            (event.position - pos).to_angle(),
                            pos.distance(event.position),
                            0.8,
                            0.8,
                            event.clone(),
                        ));
                    }
                }
                EventType::Visual(_) => {
                    senses.push(Sense::new(
                        (event.position - pos).to_angle(),
                        pos.distance(event.position),
                        0.95,
                        0.9,
                        event.clone(),
                    ));
                }
                _ => (),
            }
        }

        let (map_size, grid_size, tile_size, map_type, anchor) =
            q_tilemap.get(physics_tilemap.0).unwrap();

        predator.hunger += 0.1 * time.delta_secs();
        predator.hunger = predator.hunger.min(1.0);
        senses.push(Sense::internal(SenseEvent::internal(EventType::Internal(
            InternalEventType::Hunger {
                intensity: predator.hunger,
            },
        ))));
        senses.push(Sense::internal(SenseEvent::internal(EventType::Internal(
            InternalEventType::Movement {
                amount: forces.linear_velocity() * time.delta_secs(),
            },
        ))));
        senses.push(Sense::internal(SenseEvent::internal(EventType::Internal(
            InternalEventType::Position(pos),
        ))));
        senses.push(Sense::internal(SenseEvent::internal(EventType::Internal(
            InternalEventType::Tile(
                current,
                current.center_in_world(map_size, grid_size, tile_size, map_type, anchor) * 2.0,
            ),
        ))));

        brain.apply(&mut *predator, senses, time.delta_secs());
    }
}

pub fn translate(
    mut gizmos: Gizmos,
    spatial_query: SpatialQuery,
    query: Query<(
        Entity,
        &mut Locomotor,
        &mut Pathfinding,
        &Predator,
        &Transform,
        Option<&ConnectedBodies>,
    )>,
) {
    for (entity, mut locomotor, mut pathfinding, predator, transform, connected) in query {
        const DIRECTIONS: [Dir2; 4] = [Dir2::NEG_X, Dir2::X, Dir2::NEG_Y, Dir2::Y];

        let pos = transform.translation.xy();
        let movement = predator.desired_movement;

        pathfinding.from = pos;
        pathfinding.to = pos + movement;
        let Some(direction) = pathfinding.direction else {
            continue;
        };
        let mut movement = direction * movement.length();

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

impl SmartEntity<Action<PredatorAction>> for Predator {
    fn apply(&mut self, commands: Vec<Action<PredatorAction>>, _delta: f32) {
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
                PredatorAction::Movement(movement) => movement.length(),
                _ => 0.0,
            })
            .max_by(f32::total_cmp)
            .unwrap_or(0.0);
        let mut movement = Vec2::new(0.01, 0.01);
        for command in commands {
            match command.command {
                PredatorAction::Movement(amount) => movement += amount * command.weight,
                PredatorAction::Attack => self.attacking = true,
            }
        }
        self.desired_movement = self
            .desired_movement
            .lerp(movement.normalize() * magnitude, 0.2);
    }
}

struct NoRepeatState {
    pos: Vec2,
    recent: Vec<(TilePos, Vec2)>,
    decay_timer: Timer,
}

impl NoRepeatState {
    const TILE_MEMORY: usize = 4;

    pub fn new() -> Self {
        Self {
            pos: Vec2::ZERO,
            recent: Vec::new(),
            decay_timer: Timer::from_seconds(1.0, TimerMode::Repeating),
        }
    }
}

impl Belief<SenseEvent, Action<PredatorAction>> for NoRepeatState {
    fn decay(&mut self, delta: f32) {
        self.decay_timer.tick(Duration::from_secs_f32(delta));
        if self.decay_timer.just_finished() {
            self.recent.remove(0);
        }
    }

    fn update(&mut self, sense: Sense<SenseEvent>) {
        if let EventType::Internal(InternalEventType::Position(pos)) = sense.sense_type.event_type {
            self.pos = pos;
        }

        let EventType::Internal(InternalEventType::Tile(tile, pos)) = sense.sense_type.event_type
        else {
            return;
        };

        match self.recent.last() {
            Some((last, _)) => {
                if *last != tile {
                    self.decay_timer.finish();
                    if self.recent.len() == Self::TILE_MEMORY {
                        self.recent.remove(0);
                    }
                    self.recent.push((tile, pos));
                }
            }
            None => {
                self.decay_timer.finish();
                self.recent.push((tile, pos))
            }
        }
    }

    fn minimize(&mut self) -> Vec<Action<PredatorAction>> {
        let mut sum = Vec2::ZERO;
        for recent in &self.recent {
            sum += recent.1;
        }
        let avg: Vec2 = sum / self.recent.len() as f32 - self.pos;
        if sum == Vec2::ZERO {
            vec![]
        } else {
            vec![Action {
                command: PredatorAction::Movement(-avg.normalize() * 100.0 + Vec2::new(0.0, 50.0)), // Always want a little climbing action
                weight: 10.0,
            }]
        }
    }
}

struct AttackState {
    rng: rand::rngs::StdRng,
    prey_offset: Option<Vec2>,
    this_offset: Option<Vec2>,
    urgentness: f32,
}

impl AttackState {
    const MAX_DISTANCE: f32 = 800.0;
    const MAX_SPEED: f32 = 260.0;

    pub fn new() -> Self {
        Self {
            rng: rand::make_rng(),
            prey_offset: None,
            this_offset: None,
            urgentness: 0.0,
        }
    }
}

impl Belief<SenseEvent, Action<PredatorAction>> for AttackState {
    fn decay(&mut self, delta: f32) {
        self.urgentness -= 0.4 * delta;

        if self.urgentness <= 0.0 {
            self.urgentness = 0.0;
            self.prey_offset = None;
        }
    }

    fn update(&mut self, sense: Sense<SenseEvent>) {
        match sense.sense_type.event_type {
            EventType::Auditory {
                typ: AuditoryEventType::Scuttle,
                ..
            } => {
                let (distance, _, vector) = sense.get_pos(&mut self.rng);
                if distance > Self::MAX_DISTANCE {
                    return;
                }

                self.urgentness = 1.0;
                match &mut self.this_offset {
                    Some(offset) => {
                        if vector.length() < offset.length() {
                            *offset = vector
                        }
                    }
                    None => self.this_offset = Some(vector),
                }
            }
            _ => (),
        }
    }

    fn minimize(&mut self) -> Vec<Action<PredatorAction>> {
        if let Some(this_offset) = &self.this_offset {
            match &mut self.prey_offset {
                Some(offset) => {
                    *offset = offset.lerp(*this_offset, 0.5);
                }
                None => self.prey_offset = Some(*this_offset),
            }
        }
        self.this_offset = None;

        if let Some(offset) = self.prey_offset {
            let mut out = vec![Action {
                command: PredatorAction::Movement(
                    offset.normalize() * self.urgentness * Self::MAX_SPEED,
                ),
                weight: self.urgentness * 5.0,
            }];

            if offset.length() < 20.0 {
                out.push(Action {
                    command: PredatorAction::Attack,
                    weight: 20.0,
                });
            }

            out
        } else {
            Vec::new()
        }
    }
}
