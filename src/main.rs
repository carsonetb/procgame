use std::{collections::HashSet, time::Duration};

use avian2d::prelude::*;
use bevy::{
    camera::{RenderTarget, visibility::RenderLayers},
    dev_tools::fps_overlay::FpsOverlayPlugin,
    diagnostic::{EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    image::ImageSampler,
    input::common_conditions::{input_just_pressed, input_pressed},
    prelude::*,
    render::render_resource::{
        Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
    },
    time::common_conditions::on_timer,
    window::WindowResized,
};
use bevy_ecs_tilemap::prelude::*;
// use bevy_embedded_assets::EmbeddedAssetPlugin;

use crate::{
    body::{
        BodyHead, BodyLegs, ConnectedBodies, Elbow, Facing, Leg, Legged, Locomotor,
        LocomotorOrchestrator, PreviousVelocity, UserControlled,
    },
    environment::{AuditoryEventType, Emitter, EventType, SenseEvent},
    food::Food,
    pathfind::Pathfinding,
    predator::Predator,
    tilemap::{CurrentTile, MapDepth},
};

mod body;
mod brain;
mod creature;
mod editor;
mod environment;
mod food;
mod pathfind;
mod plants;
mod predator;
mod prerender;
mod tilemap;

const PIXEL_SCALE: f32 = 1.5;

#[derive(Resource)]
struct PixelRenderTarget(Handle<Image>);

#[derive(Component)]
struct MainCamera;

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>, window: Query<&Window>) {
    let Ok(window) = window.single() else {
        return;
    };

    let width = (window.width() / PIXEL_SCALE).ceil() as u32;
    let height = (window.height() / PIXEL_SCALE).ceil() as u32;

    let image = Image {
        texture_descriptor: TextureDescriptor {
            label: Some("Render Target"),
            size: Extent3d {
                width: width.max(1),
                height: height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Bgra8UnormSrgb,
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_DST
                | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        },
        sampler: ImageSampler::linear(),
        ..default()
    };
    let image_handle = images.add(image);

    commands.insert_resource(PixelRenderTarget(image_handle.clone()));

    commands.spawn((
        Camera2d,
        Camera {
            order: -1,
            ..default()
        },
        MainCamera,
        RenderTarget::Image(image_handle.clone().into()),
        Projection::Orthographic(OrthographicProjection {
            scale: PIXEL_SCALE,
            ..OrthographicProjection::default_2d()
        }),
        GlobalTransform::from(Transform::from_xyz(0.0, 0.0, 0.0)),
        Msaa::Off,
    ));

    commands.spawn((
        Camera2d,
        Camera {
            order: 0,
            ..default()
        },
        RenderLayers::layer(1),
    ));

    commands.spawn((
        Sprite::from_image(image_handle),
        Transform::from_scale(Vec3::splat(PIXEL_SCALE)),
        RenderLayers::layer(1),
    ));
}

fn resize_render_target(
    mut resize_events: MessageReader<WindowResized>,
    mut images: ResMut<Assets<Image>>,
    target: Res<PixelRenderTarget>,
) {
    for event in resize_events.read() {
        if let Some(image) = images.get_mut(&target.0) {
            let width = ((event.width / PIXEL_SCALE).ceil() as u32).max(1);
            let height = ((event.height / PIXEL_SCALE).ceil() as u32).max(1);

            image.resize(Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            });
        }
    }
}

fn create_foods(mut commands: Commands) {
    Food::setup(
        &mut commands,
        20.0,
        Color::srgb(0.0, 0.8, 0.3),
        Vec2::new(250.0, 250.0),
    );
    Food::setup(
        &mut commands,
        20.0,
        Color::srgb(0.0, 0.8, 0.3),
        Vec2::new(-250.0, 250.0),
    );
    Food::setup(
        &mut commands,
        20.0,
        Color::srgb(0.0, 0.8, 0.3),
        Vec2::new(-250.0, -250.0),
    );
    Food::setup(
        &mut commands,
        20.0,
        Color::srgb(0.0, 0.8, 0.3),
        Vec2::new(500.0, 250.0),
    );
    Food::setup(
        &mut commands,
        20.0,
        Color::srgb(0.0, 0.8, 0.3),
        Vec2::new(-500.0, 100.0),
    );
}

fn spawn_event(
    mut commands: Commands,
    window_query: Query<&Window>,
    camera_query: Query<(&Camera, &mut GlobalTransform), With<MainCamera>>,
    tilemap_query: Query<(
        &TilemapSize,
        &TilemapGridSize,
        &TilemapTileSize,
        &TilemapType,
        &TilemapAnchor,
        &MapDepth,
    )>,
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

    let (map_size, grid_size, tile_size, map_type, anchor, depth) = tilemap_query
        .iter()
        .find(|(_, _, _, _, _, depth)| depth.0 == 1)
        .unwrap();
    let map_pos = TilePos::from_world_pos(
        &(cursor / 2.0),
        map_size,
        grid_size,
        tile_size,
        map_type,
        anchor,
    )
    .unwrap();
    let mut tiles = HashSet::new();
    tiles.insert(map_pos);

    commands.spawn(SenseEvent::new(
        EventType::Auditory {
            typ: AuditoryEventType::Scuttle,
            affects: tiles,
        },
        cursor,
        1.0,
        5.0,
    ));
}

fn spawn_enemy(
    mut commands: Commands,
    window_query: Query<&Window>,
    camera_query: Query<(&Camera, &mut GlobalTransform), With<MainCamera>>,
    tilemap_query: Query<(Entity, &MapDepth)>,
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

    let (tilemap, _) = tilemap_query
        .iter()
        .find(|(_, depth)| depth.0 == 1)
        .unwrap();

    let legs = commands
        .spawn((
            PreviousVelocity(Vec2::ZERO),
            Legged {
                facing: Facing::Left,
                legs: vec![
                    Leg::new(
                        vec![Vec2::new(0.4, -1.0), Vec2::new(0.5, -0.5)],
                        Vec2::new(0.3, -1.0),
                        35.0,
                        160.0,
                        16.0,
                        30.0,
                        Elbow::Down,
                    ),
                    Leg::new(
                        vec![Vec2::new(-0.2, -1.0), Vec2::new(0.5, -0.5)],
                        Vec2::new(-0.3, -1.0),
                        35.0,
                        160.0,
                        16.0,
                        30.0,
                        Elbow::Down,
                    ),
                ],
            },
            Locomotor {
                desired_velocity: Vec2::new(0.0, 0.0),
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
            BodyHead,
            CurrentTile::new(tilemap),
            Predator::new(),
            Predator::brain(),
            Pathfinding::new(tilemap, cursor, cursor, 4),
            Legged {
                facing: Facing::Left,
                legs: vec![
                    Leg::new(
                        vec![
                            Vec2::new(-0.8, 0.5),
                            Vec2::new(0.0, 1.0),
                            Vec2::new(0.5, 0.5),
                        ],
                        Vec2::new(-0.7, -1.0),
                        40.0,
                        80.0,
                        8.0,
                        33.0,
                        Elbow::Up,
                    ),
                    Leg::new(
                        vec![
                            Vec2::new(-0.5, 0.5),
                            Vec2::new(0.0, 1.0),
                            Vec2::new(0.8, 0.5),
                        ],
                        Vec2::new(0.7, -1.0),
                        40.0,
                        80.0,
                        8.0,
                        33.0,
                        Elbow::Up,
                    ),
                ],
            },
            Locomotor {
                desired_velocity: Vec2::new(0.0, 0.0),
            },
            LocomotorOrchestrator(vec![legs]),
            RigidBody::Dynamic,
            Restitution::new(0.1),
            Mass(0.5),
            Collider::circle(10.0),
            Transform::from_xyz(cursor.x, cursor.y, 0.0),
            ConnectedBodies(vec![legs]),
        ))
        .id();

    commands.entity(legs).insert(ConnectedBodies(vec![arms]));

    commands.spawn(DistanceJoint::new(legs, arms).with_limits(0.0, 20.0));
}

fn spawn_rigidbody(
    mut commands: Commands,
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
                        Vec2::new(0.3, -1.0),
                        25.0,
                        160.0,
                        16.0,
                        22.0,
                        Elbow::Down,
                    ),
                    Leg::new(
                        vec![Vec2::new(-0.2, -1.0), Vec2::new(0.5, -0.5)],
                        Vec2::new(-0.3, -1.0),
                        25.0,
                        160.0,
                        16.0,
                        22.0,
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
                        Vec2::new(-0.7, -1.0),
                        30.0,
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
                        Vec2::new(0.7, -1.0),
                        30.0,
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

    commands.entity(legs).insert(ConnectedBodies(vec![arms]));

    commands.spawn(DistanceJoint::new(legs, arms).with_limits(0.0, 20.0));
}

fn camera_movement(
    time: Res<Time>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    query: Query<&mut Transform, With<Camera>>,
) {
    for mut transform in query {
        let mut direction = Vec3::ZERO;
        if keyboard_input.pressed(KeyCode::ArrowLeft) {
            direction += Vec3::new(-1.0, 0.0, 0.0);
        }
        if keyboard_input.pressed(KeyCode::ArrowRight) {
            direction += Vec3::new(1.0, 0.0, 0.0);
        }
        if keyboard_input.pressed(KeyCode::ArrowUp) {
            direction += Vec3::new(0.0, 1.0, 0.0);
        }
        if keyboard_input.pressed(KeyCode::ArrowDown) {
            direction += Vec3::new(0.0, -1.0, 0.0);
        }
        transform.translation += direction * 500.0 * time.delta_secs();
    }
}

fn main() {
    let mut app = App::new();
    // app.add_plugins(EmbeddedAssetPlugin {
    //     mode: bevy_embedded_assets::PluginMode::ReplaceDefault,
    // });
    app.add_plugins((
        DefaultPlugins
            .set(ImagePlugin::default_nearest())
            .set(WindowPlugin {
                primary_window: Some(Window {
                    present_mode: bevy::window::PresentMode::AutoNoVsync,
                    ..default()
                }),
                ..default()
            }),
        TilemapPlugin,
        PhysicsPlugins::default(),
        bevy_spatial::AutomaticUpdate::<plants::Attractor>::new()
            .with_frequency(Duration::from_secs_f32(1.0))
            .with_transform(bevy_spatial::TransformMode::Transform)
            .with_spatial_ds(bevy_spatial::SpatialStructure::KDTree2), // PhysicsDebugPlugin,
        EntityCountDiagnosticsPlugin::default(),
        FrameTimeDiagnosticsPlugin::default(),
        LogDiagnosticsPlugin::default(),
    ));
    app.insert_resource(Gravity(Vec2::NEG_Y * 980.0));
    app.insert_resource(ClearColor(Color::srgb(0.5, 0.2, 0.2)));
    app.insert_gizmo_config(
        DefaultGizmoConfigGroup,
        GizmoConfig {
            depth_bias: 0.5,
            ..default()
        },
    );
    app.add_systems(
        Startup,
        (
            setup,
            // create_creatures,
            // create_predators,
            // create_foods,
            tilemap::setup,
            plants::setup,
        ),
    );
    app.add_systems(
        Update,
        (
            resize_render_target,
            spawn_rigidbody.run_if(input_just_pressed(MouseButton::Middle)),
            spawn_enemy.run_if(input_just_pressed(KeyCode::KeyE)),
            spawn_event.run_if(input_just_pressed(KeyCode::KeyP)),
            tilemap::colordepth.run_if(on_timer(Duration::from_secs(2))),
            body::render,
            plants::render,
            plants::debug_plants.run_if(input_pressed(KeyCode::KeyT)),
            plants::debug_attractors.run_if(input_pressed(KeyCode::ShiftLeft)),
            plants::spawn_attractor.run_if(input_just_pressed(KeyCode::KeyA)),
            plants::spawn_root.run_if(input_just_pressed(KeyCode::KeyR)),
        ),
    );
    app.add_systems(
        FixedPostUpdate,
        (
            (environment::process, environment::sound, environment::emit),
            (plants::grow, plants::kill, plants::position, plants::width),
            (
                tilemap::bitmap
                    .run_if(input_pressed(MouseButton::Left).or(input_pressed(MouseButton::Right))),
                tilemap::edit,
                tilemap::update_current_tile,
            ),
            pathfind::pathfind,
            (
                body::keyboard_movement,
                body::stand,
                body::locomote,
                body::jump,
                body::animate,
                body::balance,
                body::orchestrate,
            ),
            (predator::process, predator::attack, predator::translate),
        ),
    );
    app.run();
}

// fn main() {
//     let mut app = App::new();
//     app.add_plugins((
//         DefaultPlugins.set(ImagePlugin::default_nearest()),
//         TilemapPlugin,
//         PhysicsPlugins::default(),
//     ));
//     app.insert_resource(Gravity(Vec2::NEG_Y * 980.0));
//     app.add_systems(Startup, (prerender::setup, editor::setup).chain());
//     app.add_systems(
//         Update,
//         (
//             editor::select_atlas,
//             editor::select_layer,
//             editor::select_layer_owner,
//             editor::edit_tiles,
//             editor::own_layers,
//             (
//                 prerender::clean,
//                 prerender::bitmap_tile,
//                 prerender::render_tile,
//                 prerender::render_level,
//             )
//                 .chain()
//                 .run_if(input_just_pressed(KeyCode::Space)),
//             prerender::remove_example.run_if(input_just_pressed(KeyCode::Enter)),
//         ),
//     );
//     app.run();
// }
