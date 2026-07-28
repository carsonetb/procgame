use std::{collections::HashSet, env, time::Duration};

use avian2d::prelude::*;
use bevy::{
    camera::{RenderTarget, visibility::RenderLayers},
    feathers::{FeathersPlugins, dark_theme::create_dark_theme, theme::UiTheme, tokens},
    image::ImageSampler,
    input::common_conditions::{input_just_pressed, input_pressed},
    prelude::*,
    render::render_resource::{
        Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
    },
    sprite_render::Material2dPlugin,
    time::common_conditions::on_timer,
    window::WindowResized,
};
use bevy_ecs_tilemap::prelude::*;
// use bevy_embedded_assets::EmbeddedAssetPlugin;

use crate::{
    environment::{AuditoryEventType, EventType, SenseEvent},
    tilemap::MapDepth,
};

mod body;
mod brain;
mod bullets;
mod creature;
mod editor;
mod environment;
mod health;
mod input;
mod instance;
mod items;
mod lighting;
mod multiplayer;
mod pathfind;
mod plants;
mod player;
mod predator;
mod prerender;
mod tilemap;
mod ui;

const PIXEL_SCALE: f32 = 1.5;

#[derive(Resource)]
struct PixelRenderTarget(Handle<Image>);

#[derive(Component)]
struct MainCamera;

#[derive(Debug, Default, Clone, Copy, PhysicsLayer)]
pub enum GameLayer {
    #[default]
    Environment,
    Player,
    Entity,
    Items,
}

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
            scale: PIXEL_SCALE * (1200.0 / window.width()),
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
        Projection::Orthographic(OrthographicProjection {
            scale: 1200.0 / window.width(),
            ..OrthographicProjection::default_2d()
        }),
        RenderLayers::layer(1),
    ));

    commands.spawn((
        Sprite::from_image(image_handle),
        Transform::from_scale(Vec3::splat((1200.0 / window.width()) * PIXEL_SCALE)),
        RenderLayers::layer(1),
    ));
}

fn resize_render_target(
    mut resize_events: MessageReader<WindowResized>,
    mut images: ResMut<Assets<Image>>,
    target: Res<PixelRenderTarget>,
) {
    for event in resize_events.read() {
        if let Some(mut image) = images.get_mut(&target.0) {
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

    let (map_size, grid_size, tile_size, map_type, anchor, _depth) = tilemap_query
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

#[allow(dead_code)]
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
    let args = env::args().collect::<Vec<String>>();
    let port = args.get(1).map(|s| s.clone()).unwrap_or("3000".into());
    let target = args.get(2).map(|s| s.clone());

    let mut app = App::new();
    // app.add_plugins(EmbeddedAssetPlugin {
    //     mode: bevy_embedded_assets::PluginMode::ReplaceDefault,
    // });
    // app.add_plugins(steamworks::SteamworksPlugin::init_app(480).unwrap());
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
        multiplayer::LocalNetworkingPlugin::<multiplayer::Message>::new(port, target),
        FeathersPlugins,
        Material2dPlugin::<lighting::HeightmapMaterial>::default(),
        TilemapPlugin,
        PhysicsPlugins::default().set(PhysicsInterpolationPlugin::interpolate_all()),
        // PhysicsDebugPlugin,
        bevy_spatial::AutomaticUpdate::<plants::Attractor>::new()
            .with_frequency(Duration::from_secs_f32(1.0))
            .with_transform(bevy_spatial::TransformMode::Transform)
            .with_spatial_ds(bevy_spatial::SpatialStructure::KDTree2),
        // FpsOverlayPlugin::default(),
    ));

    app.insert_resource(Gravity(Vec2::NEG_Y * 980.0));
    app.insert_resource(ClearColor(Color::srgb(0.5, 0.5, 0.8)));
    app.insert_gizmo_config(
        DefaultGizmoConfigGroup,
        GizmoConfig {
            depth_bias: 0.5,
            ..default()
        },
    );

    let mut theme = create_dark_theme();
    *theme.color.get_mut(&tokens::WINDOW_BG).unwrap() = Color::srgba(0.5, 0.5, 0.6, 0.05);
    app.insert_resource(UiTheme(theme));

    app.add_message::<multiplayer::Message>();

    app.add_systems(
        Startup,
        (
            setup,
            ui::ui.spawn(),
            (
                tilemap::load,
                tilemap::bitmap,
                tilemap::colordepth,
                // lighting::trigger_heightmap_work,
            )
                .chain(),
            plants::setup,
            bullets::setup,
            items::setup,
            input::setup,
            player::setup,
        ),
    );

    app.add_systems(
        Update,
        (
            resize_render_target,
            spawn_event.run_if(input_just_pressed(KeyCode::KeyP)),
            tilemap::colordepth.run_if(input_just_pressed(KeyCode::KeyC)),
            (
                player::spawn_at_mouse.run_if(input_just_pressed(MouseButton::Middle)),
                player::pickup_drop,
            ),
            predator::spawn_at_mouse.run_if(input_just_pressed(KeyCode::KeyX)),
            body::render,
            (
                plants::render,
                plants::debug_plants.run_if(input_pressed(KeyCode::KeyT)),
                plants::debug_attractors.run_if(input_pressed(KeyCode::ShiftLeft)),
                plants::spawn_attractor.run_if(input_just_pressed(KeyCode::KeyQ)),
                plants::spawn_root.run_if(input_just_pressed(KeyCode::KeyR)),
            ),
            (
                lighting::trigger_heightmap_work.run_if(input_just_pressed(KeyCode::KeyH)),
                lighting::poll_heightmap_work,
            ),
            input::track_controllers,
        ),
    );

    app.add_systems(
        FixedPostUpdate,
        (
            (
                player::movement,
                (player::update_sprites, player::point_sprites).chain(),
            ),
            (
                body::stand,
                body::locomote,
                body::jump,
                body::animate,
                body::balance,
                body::orchestrate,
                body::avoid::<false>,
            ),
            (items::towards_mouse, items::update_interactions),
            (
                predator::process,
                predator::attack,
                predator::translate,
                predator::locomote,
            ),
            pathfind::pathfind,
            (bullets::gravity, bullets::travel),
            (health::damage, health::cooldown),
            (
                // plants::grow,
                plants::poll_grow_task.run_if(on_timer(Duration::from_millis(100))),
                plants::kill,
                // plants::position,
                plants::width,
            ),
            (environment::process, environment::sound, environment::emit),
            (
                tilemap::bitmap.run_if(
                    input_pressed(MouseButton::Left).or_eager(input_pressed(MouseButton::Right)),
                ),
                tilemap::edit,
                tilemap::update_current_tile,
                tilemap::save.run_if(input_pressed(KeyCode::KeyK)),
            ),
        ),
    );

    app.run();
}
