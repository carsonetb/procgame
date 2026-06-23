use avian2d::prelude::*;
use bevy::{
    camera::{RenderTarget, visibility::RenderLayers},
    image::ImageSampler,
    input::common_conditions::{input_just_pressed, input_pressed},
    prelude::*,
    render::render_resource::{
        Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
    },
    window::WindowResized,
};
use bevy_ecs_tilemap::prelude::*;
// use bevy_embedded_assets::EmbeddedAssetPlugin;

use crate::{
    body::{
        BodyHead, BodyLegs, ConnectedBodies, Elbow, Facing, Leg, Legged, Locomotor,
        LocomotorOrchestrator, PreviousVelocity,
    },
    creature::Prey,
    environment::{AuditoryEventType, EventType, SenseEvent},
    food::Food,
    predator::Predator,
};

mod body;
mod brain;
mod creature;
mod editor;
mod environment;
mod food;
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

fn create_creatures(mut commands: Commands) {
    Prey::setup(&mut commands, Vec2::new(0.0, 0.0));
    Prey::setup(&mut commands, Vec2::new(100.0, 0.0));
    Prey::setup(&mut commands, Vec2::new(300.0, 0.0));
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
    commands.spawn(SenseEvent::new(
        EventType::Auditory(AuditoryEventType::Scuttle),
        cursor,
        1.0,
        5.0,
    ));
}

fn spawn_enemy(
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
            RigidBody::Dynamic,
            Restitution::new(0.1),
            Mass(1.0),
            Collider::circle(10.0),
            Transform::from_xyz(cursor.x, cursor.y, 0.0),
        ))
        .id();

    let arms = commands
        .spawn((
            Predator::new(),
            Predator::brain(),
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
        DefaultPlugins.set(ImagePlugin::default_nearest()),
        TilemapPlugin,
        PhysicsPlugins::default(),
        // PhysicsDebugPlugin,
    ));
    app.insert_resource(Gravity(Vec2::NEG_Y * 980.0));
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
        ),
    );
    app.add_systems(
        Update,
        (
            resize_render_target,
            spawn_rigidbody.run_if(input_just_pressed(MouseButton::Middle)),
            spawn_enemy.run_if(input_just_pressed(KeyCode::KeyE)),
            spawn_event.run_if(input_just_pressed(KeyCode::KeyP)),
            // update_rigidbodies,
            // camera_movement,
            tilemap::depth,
            body::render,
            // Prey::process,
            // Prey::eat,
            // Predator::process,
            // Predator::attack,
            // Food::process,
        ),
    );
    app.add_systems(
        FixedPostUpdate,
        (
            environment::process,
            tilemap::bitmap
                .run_if(input_pressed(MouseButton::Left).or(input_pressed(MouseButton::Right))),
            tilemap::edit,
            body::keyboard_movement,
            body::stand,
            body::locomote,
            body::jump,
            body::animate,
            body::balance,
            body::orchestrate,
            predator::process,
            predator::attack,
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
