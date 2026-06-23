use avian2d::prelude::*;
use bevy::prelude::*;

use crate::{body::*, brain::*, creature::Prey, environment::*};

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
        Brain::new(vec![Box::new(AttackState::new())])
    }
}

// pub fn setup(commands: &mut Commands, pos: Vec2) {
//     commands.spawn((
//         Self {
//             desired_movement: Vec2::ZERO,
//             hunger: 0.0,
//             attacking: false,
//             walk_timer: Timer::from_seconds(0.1, TimerMode::Repeating),
//         },
//         Brain::new(vec![Box::new(AttackState::new())]),
//         Sprite::from_color(Color::srgb(1.0, 0.0, 0.0), Vec2::new(30.0, 30.0)),
//         Transform::from_xyz(pos.x, pos.y, 0.0),
//     ));
// }

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
        &mut Locomotor,
        &mut Predator,
        &mut Brain<SenseEvent, Action<PredatorAction>>,
        &Transform,
    )>,
) {
    // TODO: Unify a lot of this logic into a helper.
    for (forces, mut locomotor, mut predator, mut brain, transform) in predator_query {
        let pos = transform.translation.xy();
        predator.walk_timer.tick(time.delta());
        if predator.walk_timer.just_finished() {
            commands.spawn(SenseEvent::new(
                EventType::Auditory(AuditoryEventType::Walk),
                pos,
                0.7,
                5.0,
            ));
        }

        let mut senses = Vec::new();
        for event in events_query {
            match event.event_type {
                EventType::Auditory(_) => {
                    senses.push(Sense::new(
                        (event.position - pos).to_angle(),
                        pos.distance(event.position),
                        0.8,
                        0.8,
                        *event,
                    ));
                }
                EventType::Visual(_) => {
                    senses.push(Sense::new(
                        (event.position - pos).to_angle(),
                        pos.distance(event.position),
                        0.95,
                        0.9,
                        *event,
                    ));
                }
                _ => (),
            }
        }

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

        brain.apply(&mut *predator, senses, time.delta_secs());

        locomotor.desired_velocity = predator.desired_movement;
    }
}

impl SmartEntity<Action<PredatorAction>> for Predator {
    fn apply(&mut self, commands: Vec<Action<PredatorAction>>, delta: f32) {
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
            EventType::Auditory(AuditoryEventType::Scuttle) => {
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
