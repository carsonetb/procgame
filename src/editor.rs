#![allow(dead_code)]

use std::collections::HashMap;

use crate::prerender::*;
use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;

const LEVEL_WIDTH: u32 = 40;
const LEVEL_HEIGHT: u32 = 40;

#[derive(Resource, Debug, Clone, Copy)]
pub struct SelectedAtlas(AtlasID);

#[derive(Component, Debug, Clone, Copy)]
pub struct SelectedAtlasDisplay(AtlasID);

#[derive(Component, Debug, Clone, Copy)]
pub struct SelectedLayer {
    layer: i32,
    created: bool,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct SelectedLayerOwner(i32);

fn spawn_tile(
    commands: &mut Commands,
    storage: &mut TileStorage,
    layer: &mut Layer,
    x: u32,
    y: u32,
    atlas: AtlasID,
    tilemap: Entity,
) -> Entity {
    let pos = TilePos { x, y };
    let entity = commands
        .spawn((
            TileBundle {
                position: pos,
                tilemap_id: TilemapId(tilemap),
                texture_index: TileTextureIndex(0),
                ..Default::default()
            },
            Tile {
                atlas,
                layer: tilemap,
                atlas_pos: UVec2::new(0, 0),
                tile_pos: UVec2::new(x, y),
            },
        ))
        .id();
    storage.set(&pos, entity);
    layer.tiles.set(x, y, Some(entity));
    entity
}

fn create_layer(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    atlas: AtlasID,
    layer: i32,
    owner: i32,
    width: u32,
    height: u32,
    tile_width: u32,
    tile_height: u32,
) {
    let tilemap_entity = commands.spawn_empty().id();

    let map_size = TilemapSize {
        x: width,
        y: height,
    };
    let tile_size = TilemapTileSize {
        x: tile_width as f32,
        y: tile_height as f32,
    };

    let mut layer = Layer {
        tiles: Matrix::new(width, height, None),
        dimensions: UVec2::new(width, height),
        layer,
        owner,
        output: Matrix::new(width * tile_width, height * tile_height, Color::NONE),
        following: false,
    };

    let mut tile_storage = TileStorage::empty(map_size);
    for x in 0..width {
        for y in 0..height {
            spawn_tile(
                commands,
                &mut tile_storage,
                &mut layer,
                x,
                y,
                atlas,
                tilemap_entity,
            );
        }
    }

    let debug_image = asset_server.load("debug_map.png");
    let tilemap = TilemapBundle {
        grid_size: tile_size.into(),
        map_type: TilemapType::Square,
        size: map_size,
        storage: tile_storage,
        texture: TilemapTexture::Single(debug_image),
        tile_size,
        anchor: TilemapAnchor::Center,
        transform: Transform::from_scale(Vec3::splat(1.5)),
        ..Default::default()
    };

    commands.entity(tilemap_entity).insert((tilemap, layer));
}

pub fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut atlas_db: ResMut<AtlasDB>,
) {
    commands.spawn(Camera2d);

    let image = asset_server.load("tilemap.png");
    let mut bitmap = HashMap::new();
    bitmap.insert(BitDirection::RIGHT | BitDirection::DOWN, UVec2::new(1, 1));
    bitmap.insert(
        BitDirection::RIGHT | BitDirection::DOWN | BitDirection::LEFT,
        UVec2::new(2, 1),
    );
    bitmap.insert(BitDirection::LEFT | BitDirection::DOWN, UVec2::new(3, 1));
    bitmap.insert(
        BitDirection::UP | BitDirection::DOWN | BitDirection::RIGHT,
        UVec2::new(1, 2),
    );
    bitmap.insert(
        BitDirection::UP | BitDirection::DOWN | BitDirection::LEFT | BitDirection::RIGHT,
        UVec2::new(2, 2),
    );
    bitmap.insert(
        BitDirection::UP | BitDirection::DOWN | BitDirection::LEFT,
        UVec2::new(3, 2),
    );
    bitmap.insert(BitDirection::UP | BitDirection::RIGHT, UVec2::new(1, 3));
    bitmap.insert(
        BitDirection::UP | BitDirection::LEFT | BitDirection::RIGHT,
        UVec2::new(2, 3),
    );
    bitmap.insert(BitDirection::UP | BitDirection::LEFT, UVec2::new(3, 3));
    let atlas = atlas_db.add(Atlas {
        image: image.clone(),
        dimensions: UVec2::new(40, 40),
        tile_dimensions: UVec2::new(20, 20),
        bitmap: bitmap.clone(),
        fallback: UVec2::new(0, 0),
    });
    commands.spawn((
        SelectedAtlasDisplay(atlas),
        Sprite::from_image(image),
        Transform::from_xyz(-780.0, 480.0, 0.0).with_scale(Vec3::splat(2.0)),
    ));
    commands.insert_resource(SelectedAtlas(atlas));

    create_layer(&mut commands, &asset_server, atlas, 0, 0, 40, 40, 20, 20);

    let image = asset_server.load("tilemap_background1.png");
    atlas_db.add(Atlas {
        image,
        dimensions: UVec2::new(40, 40),
        tile_dimensions: UVec2::new(20, 20),
        bitmap: bitmap.clone(),
        fallback: UVec2::new(0, 0),
    });

    commands.spawn((
        SelectedLayer {
            layer: 0,
            created: true,
        },
        Text2d::new("Selected Layer: 0"),
        TextColor(Color::WHITE),
        Transform::from_xyz(-780.0, 350.0, 0.0),
    ));

    commands.spawn((
        SelectedLayerOwner(0),
        Text2d::new("Layer Onwer: 0"),
        TextColor(Color::WHITE),
        Transform::from_xyz(-780.0, 310.0, 0.0),
    ));
}

pub fn select_atlas(
    input: Res<ButtonInput<KeyCode>>,
    mut selected_atlas: ResMut<SelectedAtlas>,
    atlas_db: Res<AtlasDB>,
    mut display: Query<(&mut Sprite, &mut SelectedAtlasDisplay)>,
) {
    let Ok((mut display, mut display_id)) = display.single_mut() else {
        return error!("No atlas selection display!");
    };

    if input.just_pressed(KeyCode::ArrowLeft) && selected_atlas.0.0 > 0 {
        selected_atlas.0.0 -= 1;
    }
    if input.just_pressed(KeyCode::ArrowRight) && selected_atlas.0.0 < atlas_db.total() - 1 {
        selected_atlas.0.0 += 1;
    }

    if display_id.0 != selected_atlas.0 {
        display_id.0 = selected_atlas.0;
        display.image = atlas_db.get(display_id.0).unwrap().image.clone();
    }
}

pub fn select_layer(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    selected_atlas: Res<SelectedAtlas>,
    input: Res<ButtonInput<KeyCode>>,
    mut layers: Query<(&Layer, &mut Visibility)>,
    mut selected_layer: Query<(&mut SelectedLayer, &mut Text2d, &mut TextColor)>,
) {
    let (mut selected_layer, mut text, mut color) = selected_layer.single_mut().unwrap();

    let mut changed = false;
    if input.just_pressed(KeyCode::ArrowDown) {
        selected_layer.layer -= 1;
        changed = true;
    }
    if input.just_pressed(KeyCode::ArrowUp) {
        selected_layer.layer += 1;
        changed = true;
    }
    if changed {
        for (_, mut visibility) in layers.iter_mut() {
            *visibility = Visibility::Hidden;
        }

        let layer = layers
            .iter_mut()
            .find(|(layer, _)| layer.layer == selected_layer.layer);
        selected_layer.created = layer.is_some();
        text.0 = format!("Selected Layer: {}", selected_layer.layer);
        color.0 = if selected_layer.created {
            Color::WHITE
        } else {
            Color::srgb(0.5, 0.5, 0.5)
        };

        if let Some((_, mut visibility)) = layer {
            *visibility = Visibility::Visible;
        }
    }

    if input.just_pressed(KeyCode::Enter) && !selected_layer.created {
        create_layer(
            &mut commands,
            &asset_server,
            selected_atlas.0,
            selected_layer.layer,
            selected_layer.layer,
            LEVEL_WIDTH,
            LEVEL_HEIGHT,
            20,
            20,
        );
    }
}

pub fn select_layer_owner(
    input: Res<ButtonInput<KeyCode>>,
    layers: Query<(&Layer, &TileStorage)>,
    mut layer_owner: Query<(&mut SelectedLayerOwner, &mut Text2d, &mut TextColor)>,
) {
    let layers = layers
        .iter()
        .map(|layer| (layer.0.layer, layer))
        .collect::<HashMap<_, _>>();
    let (mut layer_owner, mut text, mut color) = layer_owner.single_mut().unwrap();

    let mut changed = false;
    if input.just_pressed(KeyCode::BracketLeft) {
        layer_owner.0 -= 1;
        changed = true;
    }
    if input.just_pressed(KeyCode::BracketRight) {
        layer_owner.0 += 1;
        changed = true;
    }

    if changed {
        text.0 = format!("Layer Owner: {}", layer_owner.0);
        color.0 = if layers.get(&layer_owner.0).is_some() {
            Color::WHITE
        } else {
            Color::srgb(1.0, 0.0, 0.0)
        };
    }
}

pub fn edit_tiles(
    mut commands: Commands,
    selected_owner: Query<&SelectedLayerOwner>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    selected_atlas: Res<SelectedAtlas>,
    tilemap_query: Query<
        (
            Entity,
            &mut TileStorage,
            &mut Layer,
            &TilemapSize,
            &TilemapGridSize,
            &TilemapTileSize,
            &TilemapType,
            &TilemapAnchor,
            &GlobalTransform,
            &Visibility,
        ),
        Without<Camera2d>,
    >,
    window_query: Query<&Window>,
    camera_query: Query<(&Camera, &mut GlobalTransform), With<Camera2d>>,
) {
    let selected_owner = selected_owner.single().unwrap();
    let Ok(window) = window_query.single() else {
        return;
    };
    let Ok((camera, camera_transform)) = camera_query.single() else {
        return;
    };
    let Some(cursor) = window
        .cursor_position()
        .and_then(|cursor| camera.viewport_to_world_2d(camera_transform, cursor).ok())
    else {
        return;
    };

    for (
        tilemap,
        mut storage,
        mut layer,
        map_size,
        grid_size,
        tile_size,
        map_type,
        anchor,
        transform,
        visibility,
    ) in tilemap_query
    {
        if *visibility != Visibility::Hidden {
            layer.owner = selected_owner.0;
        }

        if *visibility == Visibility::Hidden || layer.owner != layer.layer {
            continue;
        }

        if let Some(position) = TilePos::from_world_pos(
            &(cursor / transform.scale().xy()),
            map_size,
            grid_size,
            tile_size,
            map_type,
            anchor,
        ) {
            if mouse_input.pressed(MouseButton::Right)
                && let Some(entity) = storage.get(&position)
            {
                commands.entity(entity).despawn();
                storage.remove(&position);
                *layer.tiles.get_mut(position.x, position.y).unwrap() = None;
            }
            if mouse_input.pressed(MouseButton::Left)
                && let None = storage.get(&position)
            {
                let tile_entity = spawn_tile(
                    &mut commands,
                    &mut storage,
                    &mut layer,
                    position.x,
                    position.y,
                    selected_atlas.0,
                    tilemap,
                );
                storage.set(&position, tile_entity);
            }
        }
    }
}

pub fn own_layers(mut commands: Commands, query: Query<(&mut Layer, &mut TileStorage)>) {
    let ids: HashMap<i32, (Matrix<Option<Entity>>, TileStorage)> = query
        .iter()
        .map(|(layer, storage)| (layer.layer, (layer.tiles.clone(), storage.clone())))
        .collect();

    for (mut layer, mut storage) in query {
        if layer.layer != layer.owner {
            let Some((tiles, owner_storage)) = ids.get(&layer.owner) else {
                continue;
            };

            if !layer.following {
                for tile in storage.iter() {
                    if let Some(entity) = tile {
                        commands.entity(*entity).despawn();
                    }
                }
                layer.following = true;
            }

            layer.tiles = tiles.clone();
            *storage = owner_storage.clone();
        }
    }
}
