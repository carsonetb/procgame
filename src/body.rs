use avian2d::prelude::*;
use bevy::{math::ops::atan2, prelude::*};

#[derive(Debug, Clone, Copy)]
pub enum Elbow {
    Down,
    Up,
}

#[derive(Component, Debug, Clone)]
pub struct ConnectedBodies(pub Vec<Entity>);

#[derive(Component, Debug, Clone, Copy)]
pub struct BodyHead;

#[derive(Component, Debug, Clone, Copy)]
pub struct BodyLegs;

#[derive(Component, Debug, Clone, Copy)]
pub struct PreviousVelocity(pub Vec2);

impl Elbow {
    fn with_facing(&self, facing: Facing) -> Self {
        match self {
            Self::Down => match facing {
                Facing::Left => Self::Up,
                Facing::Right => Self::Down,
            },
            Self::Up => match facing {
                Facing::Left => Self::Down,
                Facing::Right => Self::Up,
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct Leg {
    pub directions: Vec<Vec2>,
    pub hang_direction: Vec2,
    pub current_direction: Vec2,
    pub length: f32,
    pub stiffness: f32,
    pub damping: f32,
    pub hold_length: f32,
    pub elbow: Elbow,
    pub position: Vec2,
    pub hit_data: Option<RayHitData>,
    pub stepping: bool,
    pub move_target: Option<Vec2>,
}

impl Leg {
    pub fn new(
        directions: Vec<Vec2>,
        hang_direction: Vec2,
        length: f32,
        stiffness: f32,
        damping: f32,
        hold_length: f32,
        elbow: Elbow,
    ) -> Self {
        Self {
            current_direction: directions[0],
            directions,
            hang_direction,
            length,
            stiffness,
            damping,
            hold_length,
            elbow,
            position: Vec2::ZERO,
            hit_data: None,
            stepping: false,
            move_target: None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Facing {
    Left,
    Right,
}

impl Facing {
    fn scale(&self) -> Vec2 {
        match self {
            Self::Left => Vec2::new(-1.0, 1.0),
            Self::Right => Vec2::new(1.0, 1.0),
        }
    }
}

#[derive(Component, Debug, Clone)]
pub struct Legged {
    pub facing: Facing,
    pub legs: Vec<Leg>,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct Locomotor {
    pub desired_velocity: Vec2,
}

pub fn keyboard_movement(
    input: Res<ButtonInput<KeyCode>>,
    query: Query<(&mut Legged, &mut Locomotor)>,
) {
    let mut movement = Vec2::ZERO;
    if input.pressed(KeyCode::KeyA) {
        movement += Vec2::new(-100.0, 0.0);
    }
    if input.pressed(KeyCode::KeyD) {
        movement += Vec2::new(100.0, 0.0);
    }
    if input.pressed(KeyCode::KeyS) {
        movement += Vec2::new(0.0, -70.0);
    }
    if input.pressed(KeyCode::KeyW) {
        movement.y = 0.0;
        movement += Vec2::new(0.0, 100.0);
    }
    for (mut legged, mut locomotor) in query {
        locomotor.desired_velocity = movement;
        legged.facing = if movement.x < 0.0 {
            Facing::Left
        } else {
            Facing::Right
        };
    }
}

pub fn stand(
    spatial_query: SpatialQuery,
    query: Query<
        (
            Entity,
            Forces,
            &mut Legged,
            &Transform,
            Option<&ConnectedBodies>,
        ),
        With<RigidBody>,
    >,
) {
    for (entity, mut forces, mut legged, transform, connected) in query {
        let origin = transform.translation.xy();
        let scale = legged.facing.scale();
        for leg in &mut legged.legs {
            let mut excluded = vec![entity];
            if let Some(connected) = connected {
                for connected in &connected.0 {
                    excluded.push(*connected);
                }
            }

            let mut lowest_distance = f32::INFINITY;
            let mut lowest_data = None;
            for direction in &leg.directions {
                let direction = direction * scale;
                if let Some(hit_data) = spatial_query.cast_ray(
                    origin,
                    Dir2::from_xy(direction.x, direction.y).unwrap(),
                    leg.length,
                    true,
                    &SpatialQueryFilter::default().with_excluded_entities(excluded.clone()),
                ) {
                    if hit_data.distance < lowest_distance {
                        lowest_distance = hit_data.distance;
                        lowest_data = Some(hit_data);
                        leg.current_direction = direction;
                    }
                }
            }

            if let Some(hit_data) = lowest_data {
                let backwards = -leg.current_direction.normalize();
                let speed = forces.linear_velocity().dot(backwards);
                let force =
                    leg.stiffness * (leg.hold_length - hit_data.distance) - leg.damping * speed;
                if !leg.stepping {
                    forces.apply_force(backwards * force);
                }
                leg.hit_data = Some(hit_data);
            } else {
                leg.hit_data = None;
            }
        }
    }
}

pub fn balance(
    time: Res<Time>,
    query: Query<(Forces, &ConnectedBodies), With<BodyHead>>,
    mut other_query: Query<(Forces, &mut PreviousVelocity), Without<BodyHead>>,
) {
    for (mut forces, connected) in query {
        let (other_forces, mut previous) = other_query.get_mut(connected.0[0]).unwrap();
        let acceleration = previous.0 - other_forces.linear_velocity() / time.delta_secs();
        forces.apply_force(
            Vec2::new(0.0, 600.0)
                - 0.01 * acceleration
                - 2.0 * (forces.linear_velocity() - other_forces.linear_velocity()),
        );
        previous.0 = forces.linear_velocity();
    }
}

pub fn locomote(mut query: Query<(Forces, &Locomotor, &mut Legged), With<RigidBody>>) {
    for (mut forces, locomotor, mut legged) in query.iter_mut() {
        for leg in &mut legged.legs {
            if leg.stepping {
                continue;
            }

            let Some(hit_data) = leg.hit_data else {
                continue;
            };

            let leverage = 1.0 - hit_data.distance / leg.length;
            let force =
                40.0 * (locomotor.desired_velocity - forces.linear_velocity()) * (leverage - 0.05);
            forces.apply_force(force);
        }
    }
}

pub fn jump(query: Query<(Forces, &Locomotor, &Legged), With<BodyLegs>>) {
    for (mut forces, locomotor, legged) in query {
        if locomotor.desired_velocity.y > 0.0 && forces.linear_velocity().y > 0.0 {
            for leg in &legged.legs {
                if leg.stepping {
                    continue;
                }

                let Some(hit_data) = leg.hit_data else {
                    continue;
                };

                if hit_data.normal.x.abs() > 0.2 {
                    continue;
                }

                let leverage = (1.0 - (hit_data.distance / leg.length - 0.7)).max(0.0);
                forces.apply_linear_impulse(Vec2::new(
                    (forces.linear_velocity().x.abs() * 0.02).max(1.0).min(1.5)
                        * 10.0
                        * forces.linear_velocity().x.signum(),
                    (locomotor.desired_velocity.y * leverage.min(0.95)).min(60.0),
                ));
            }
        }
    }
}

pub fn animate(query: Query<(Forces, &mut Legged, &Transform), With<RigidBody>>) {
    for (forces, mut legged, transform) in query {
        let origin = transform.translation.xy();
        let mut any_stepping = legged
            .legs
            .iter()
            .any(|leg| leg.stepping && leg.move_target.is_some());
        for leg in &mut legged.legs {
            if let Some(hit_data) = leg.hit_data {
                let center =
                    origin + leg.current_direction.normalize().normalize() * hit_data.distance;

                if (leg.position.distance(center) > leg.length * 0.5
                    && !leg.stepping
                    && !any_stepping)
                    || (leg.position.distance(center) > leg.length * 0.7 && !leg.stepping)
                {
                    leg.stepping = true;
                    leg.move_target = Some(center + forces.linear_velocity() * 0.2);
                    any_stepping = true;
                }

                if leg.stepping
                    && let Some(target) = leg.move_target
                {
                    leg.position = leg.position.lerp(target, 0.2);

                    if leg.position.distance(target) < 5.0 {
                        leg.stepping = false;
                        leg.position = target;
                        leg.move_target = None;
                    }
                } else if leg.stepping {
                    leg.move_target = Some(center);
                }
            } else {
                leg.stepping = true;
                leg.move_target = None;
                leg.position = leg.position.lerp(
                    origin + leg.hang_direction.normalize() * leg.length
                        - forces.linear_velocity() * 0.05,
                    0.5,
                );
            }
        }
    }
}

pub fn render(mut gizmos: Gizmos, query: Query<(&mut Legged, &Transform), With<RigidBody>>) {
    for (mut legged, transform) in query {
        let origin = transform.translation.xy();
        gizmos.circle_2d(origin, 10.0, Color::srgb(1.0, 0.0, 1.0));
        let facing = legged.facing;
        for leg in &mut legged.legs {
            let end = leg.position;
            let distance = origin.distance(end);
            let l1 = leg.length / 2.0;
            let l2 = leg.length / 2.0;
            if distance > l1 + l2 {
                gizmos.line_2d(
                    origin,
                    origin + (end - origin).normalize() * (l1 + l2),
                    Color::srgb(1.0, 0.0, 1.0),
                );
            }
            let elbow_angle =
                ((distance.powf(2.0) - l1.powf(2.0) - l2.powf(2.0)) / (2.0 * l1 * l2)).acos()
                    * match leg.elbow.with_facing(facing) {
                        Elbow::Up => 1.0,
                        Elbow::Down => -1.0,
                    };
            let shoulder_angle = atan2(end.y - origin.y, end.x - origin.x)
                - atan2(l1 * elbow_angle.sin(), l1 + l2 * elbow_angle.cos());
            let middle = origin + Vec2::new(l1 * shoulder_angle.cos(), l2 * shoulder_angle.sin());
            gizmos.line_2d(origin, middle, Color::srgb(1.0, 0.0, 1.0));
            gizmos.line_2d(middle, end, Color::srgb(1.0, 0.0, 1.0));
        }
    }
}
