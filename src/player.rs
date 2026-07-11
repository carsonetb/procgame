use std::{collections::HashSet, f32::consts::PI};

use avian2d::prelude::*;
use bevy::prelude::*;

use crate::{MainCamera, PIXEL_SCALE, body::*, environment::*};

#[derive(Component, Default, Debug, Clone, Copy)]
pub struct Player;

#[derive(Component, Debug, Clone, Copy)]
pub struct ConnectedSprite(pub Entity);

#[derive(Component, Debug, Clone, Copy)]
pub struct PointTowards {
    pub what: Entity,
    pub backwards: bool,
    pub offset: f32,
    pub mix: Option<(Vec2, f32)>,
}

pub fn update_sprites(
    q_connected: Query<(&mut Transform, &ConnectedSprite)>,
    q_other: Query<&Transform, Without<ConnectedSprite>>,
) {
    for (mut transform, ConnectedSprite(connected)) in q_connected {
        let other_transform = q_other.get(*connected).unwrap();
        transform.translation = other_transform.translation
    }
}

pub fn point_sprites(
    q_connected: Query<(&mut Transform, &PointTowards)>,
    q_other: Query<&Transform, Without<PointTowards>>,
) {
    for (mut transform, towards) in q_connected {
        let other_transform = q_other.get(towards.what).unwrap();
        let mut offset =
            (other_transform.translation.xy() - transform.translation.xy()).normalize();
        if let Some((towards, percent)) = towards.mix {
            offset = offset.lerp(towards, percent);
        }
        transform.rotation = Quat::from_rotation_z(
            offset.y.atan2(offset.x) + PI / 2.0 * if towards.backwards { -1.0 } else { 1.0 },
        );
        let offset = -offset * towards.offset;
        transform.translation += offset.extend(0.0);
    }
}

pub fn movement(
    input: Res<ButtonInput<KeyCode>>,
    query: Query<&mut Locomotor, With<UserControlled>>,
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
    for mut locomotor in query {
        locomotor.desired_velocity = movement;
    }
}

pub fn spawn_at_mouse(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    window_query: Query<&Window>,
    camera_query: Query<(&Camera, &mut GlobalTransform), With<MainCamera>>,
) {
    let Ok(window) = window_query.single() else {
        return;
    };
    let Ok((camera, camera_transform)) = camera_query.single() else {
        return;
    };
    let Some(cursor) = window.cursor_position().and_then(|cursor| {
        camera
            .viewport_to_world_2d(camera_transform, cursor / PIXEL_SCALE)
            .ok()
    }) else {
        return;
    };

    let legs = commands
        .spawn((
            UserControlled,
            BodyLegs,
            PreviousVelocity(Vec2::ZERO),
            Legged {
                facing: Facing::Left,
                legs: vec![
                    Leg::new(
                        vec![Vec2::new(0.4, -1.0), Vec2::new(0.5, -0.5)],
                        Vec2::new(3.0, 0.0),
                        Vec2::new(0.3, -1.0),
                        28.0,
                        160.0,
                        16.0,
                        27.0,
                        Elbow::Down,
                    ),
                    Leg::new(
                        vec![Vec2::new(-0.2, -1.0), Vec2::new(0.5, -0.5)],
                        Vec2::new(-3.0, 0.0),
                        Vec2::new(-0.3, -1.0),
                        28.0,
                        160.0,
                        16.0,
                        27.0,
                        Elbow::Down,
                    ),
                ],
            },
            Locomotor {
                desired_velocity: Vec2::new(0.0, 0.0),
            },
            Emitter {
                event: SenseEvent::new(
                    EventType::Auditory {
                        typ: AuditoryEventType::Scuttle,
                        affects: HashSet::new(),
                    },
                    Vec2::ZERO,
                    0.6,
                    5.0,
                ),
                interval: Timer::from_seconds(0.5, TimerMode::Repeating),
            },
            RigidBody::Dynamic,
            Restitution::new(0.1),
            Mass(1.0),
            Collider::circle(10.0),
            Transform::from_xyz(cursor.x, cursor.y, 0.0),
        ))
        .id();

    let arms = commands
        .spawn((
            UserControlled,
            BodyHead,
            Legged {
                facing: Facing::Left,
                legs: vec![
                    Leg::new(
                        vec![
                            Vec2::new(-0.8, 0.5),
                            Vec2::new(0.0, 1.0),
                            Vec2::new(0.5, 0.5),
                        ],
                        Vec2::new(-2.0, 0.0),
                        Vec2::new(-0.7, -1.0),
                        35.0,
                        80.0,
                        8.0,
                        22.0,
                        Elbow::Up,
                    ),
                    Leg::new(
                        vec![
                            Vec2::new(-0.5, 0.5),
                            Vec2::new(0.0, 1.0),
                            Vec2::new(0.8, 0.5),
                        ],
                        Vec2::new(2.0, 0.0),
                        Vec2::new(0.7, -1.0),
                        35.0,
                        80.0,
                        8.0,
                        22.0,
                        Elbow::Up,
                    ),
                ],
            },
            Locomotor {
                desired_velocity: Vec2::new(0.0, 0.0),
            },
            RigidBody::Dynamic,
            Restitution::new(0.1),
            Mass(0.5),
            Collider::circle(10.0),
            Transform::from_xyz(cursor.x, cursor.y, 0.0),
            ConnectedBodies(vec![legs]),
        ))
        .id();

    commands.spawn((
        ConnectedSprite(arms),
        PointTowards {
            what: legs,
            backwards: false,
            offset: 5.0,
            mix: Some((Vec2::NEG_Y, 0.2)),
        },
        Sprite::from_image(asset_server.load("character_head.png")),
        Transform::from_scale(Vec3::splat(2.0)),
    ));

    commands.spawn((
        ConnectedSprite(legs),
        PointTowards {
            what: arms,
            backwards: true,
            offset: 5.0,
            mix: None,
        },
        Sprite::from_image(asset_server.load("character_torso.png")),
        Transform::from_scale(Vec3::splat(2.0)),
    ));

    commands.entity(legs).insert(ConnectedBodies(vec![arms]));

    commands.spawn(DistanceJoint::new(legs, arms).with_limits(28.0, 30.0));
}
