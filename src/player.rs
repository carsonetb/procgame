use std::{
    collections::{HashMap, HashSet},
    f32::consts::PI,
};

use avian2d::prelude::*;
use bevy::prelude::*;

use crate::{
    GameLayer, MainCamera, PIXEL_SCALE,
    body::*,
    environment::*,
    input::{Controllers, get_action},
    instance::Instance,
    items::*,
};

const PICKUP_OFFSET: Vec2 = Vec2::new(35.0, 50.0);

#[derive(Resource, Debug, Default, Deref, DerefMut, Clone)]
pub struct PlayerIndices(HashMap<u32, Vec<Entity>>);

impl PlayerIndices {
    fn get_index(&self) -> u32 {
        let mut i = 0;
        while self.contains_key(&i) {
            i += 1;
        }
        i
    }
}

#[derive(Component, Default, Debug, Clone, Copy)]
pub struct Player(pub u32);

#[derive(Component, Debug, Clone, Copy)]
pub struct ConnectedSprite(pub Instance<Transform>);

#[derive(Component, Debug, Clone, Copy)]
pub struct PointTowards {
    pub what: Entity,
    pub backwards: bool,
    pub offset: f32,
    pub mix: Option<(Vec2, f32)>,
}

pub fn setup(mut commands: Commands) {
    commands.insert_resource(PlayerIndices::default());
}

pub fn movement(
    input: Res<ButtonInput<KeyCode>>,
    controllers: Res<Controllers>,
    q_gamepads: Query<&Gamepad>,
    query: Query<(&mut Locomotor, &Player)>,
) {
    for (mut locomotor, &Player(index)) in query {
        let action = get_action(&input, &controllers, &q_gamepads, index as usize);
        let mut movement = Vec2::ZERO;
        movement.x += action.horizontal * 100.0;
        movement.y += action.vertical * 100.0;

        locomotor.desired_velocity = movement;
    }
}

pub fn pickup_drop(
    input: Res<ButtonInput<KeyCode>>,
    controllers: Res<Controllers>,
    q_gamepads: Query<&Gamepad>,
    q_player: Query<(&mut Legged, &Transform, &Player), With<BodyHead>>,
    mut q_item: Query<(Entity, &mut Item, &mut LinearVelocity, &Transform)>,
) {
    for (mut legged, player_transform, &Player(index)) in q_player {
        let action = get_action(&input, &controllers, &q_gamepads, index as usize);

        if !action.grab_drop {
            continue;
        }

        if let Some(holding) = &legged.holding {
            let (_, mut item, mut velocity, transform) =
                q_item.get_mut(holding.holding.entity).unwrap();
            item.held = false;
            velocity.0 = Vec2::from_angle(transform.rotation.to_euler(EulerRot::XYZ).2) * 800.0;
            legged.holding = None;
            continue;
        }

        for (entity, mut item, _, item_transform) in &mut q_item {
            let offset = item_transform.translation.xy() - player_transform.translation.xy();
            if offset.x.abs() < PICKUP_OFFSET.x && offset.y.abs() < PICKUP_OFFSET.y {
                item.held = true;
                legged.holding = Some(Holding {
                    holding: Instance::from(entity),
                    offset: Vec2::new(0.0, -20.0),
                    hands: 2,
                });
                break;
            }
        }
    }
}

pub fn update_sprites(
    q_connected: Query<(&mut Transform, &ConnectedSprite)>,
    q_other: Query<&Transform, Without<ConnectedSprite>>,
) {
    for (mut transform, ConnectedSprite(connected)) in q_connected {
        let other_transform = q_other.get(connected.entity).unwrap();
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

pub fn spawn_at_mouse(
    mut commands: Commands,
    mut player_indices: ResMut<PlayerIndices>,
    asset_server: Res<AssetServer>,
    item_assets: Res<ItemAssets>,
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

    let stick = build_stick(&mut commands, &item_assets, Vec2::default());

    let index = player_indices.get_index();

    let legs = commands
        .spawn((
            Player(index),
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
                holding: None,
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
            CollisionLayers::new(
                [GameLayer::Player, GameLayer::Entity],
                [GameLayer::Environment],
            ),
            Restitution::new(0.1),
            Mass(1.0),
            Collider::circle(10.0),
            Transform::from_xyz(cursor.x, cursor.y, 0.0),
        ))
        .id();

    let arms = commands
        .spawn((
            Player(index),
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
                holding: Some(Holding {
                    holding: stick,
                    offset: Vec2::new(0.0, -20.0),
                    hands: 2,
                }),
            },
            Locomotor {
                desired_velocity: Vec2::new(0.0, 0.0),
            },
            RigidBody::Dynamic,
            CollisionLayers::new(
                [GameLayer::Player, GameLayer::Entity],
                [GameLayer::Environment],
            ),
            Restitution::new(0.1),
            Mass(0.5),
            Collider::circle(10.0),
            Transform::from_xyz(cursor.x, cursor.y, 0.0),
            ConnectedBodies(vec![Instance::from(legs)]),
        ))
        .id();

    player_indices.insert(index, vec![arms, legs]);

    info!("Spawn player with index {index}");

    commands.spawn((
        ConnectedSprite(Instance::from(arms)),
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
        ConnectedSprite(Instance::from(legs)),
        PointTowards {
            what: arms,
            backwards: true,
            offset: 5.0,
            mix: None,
        },
        Sprite::from_image(asset_server.load("character_torso.png")),
        Transform::from_scale(Vec3::splat(2.0)),
    ));

    commands
        .entity(legs)
        .insert(ConnectedBodies(vec![Instance::from(arms)]));

    commands.spawn(DistanceJoint::new(legs, arms).with_limits(28.0, 30.0));
}
