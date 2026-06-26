use std::collections::HashMap;

use avian2d::prelude::*;
use bevy::prelude::*;
use bevy_ecs_tilemap::{helpers::square_grid::neighbors::Neighbors, prelude::*};
use phf_macros::phf_map;
use rand::{RngExt, seq::IndexedRandom};

use crate::{MainCamera, PIXEL_SCALE};

const TO_BITMAP: phf::Map<(i32, i32), u8> = phf_map! {
    // (-1, 1) =>  0b10000000,
    (0, 1) =>  0b1000,
    // (1, 1) =>   0b00100000,
    (-1, 0) =>  0b0100,
    (1, 0) =>   0b0010,
    // (-1, -1) => 0b00000100,
    (0, -1) =>   0b0001,
    // (1, -1) =>  0b00000001,
};

#[derive(Resource, Debug, Clone)]
pub struct TilemapGroups(pub Vec<Entity>);

#[derive(Resource, Debug, Clone, Copy)]
pub struct PhysicsTilemap(pub Entity);

#[derive(Component, Debug, Clone, Copy)]
pub enum MapType {
    Command,
    Companion(Entity),
}

#[derive(Component, Debug, Clone, Copy)]
pub struct MapDepth(pub i32);

#[derive(Component, Clone)]
pub struct BitMap {
    map: HashMap<u8, Vec<TileTextureIndex>>,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct CurrentTile {
    tilemap: Entity,
    pub tile: Option<TilePos>,
}

impl CurrentTile {
    pub fn new(tilemap: Entity) -> Self {
        Self {
            tilemap,
            tile: None,
        }
    }
}

fn spawn_tile(
    commands: &mut Commands,
    tilemap: Entity,
    typ: MapType,
    map_size: TilemapSize,
    tile_size: TilemapTileSize,
    tile_pos: TilePos,
    physics: bool,
) -> Entity {
    let world = tile_pos.center_in_world(
        &map_size,
        &TilemapGridSize::new(tile_size.x, tile_size.y),
        &tile_size,
        &TilemapType::Square,
        &TilemapAnchor::Center,
    ) * 2.0;
    match (typ, physics) {
        (MapType::Command, true) => commands
            .spawn((
                TileBundle {
                    position: tile_pos,
                    tilemap_id: TilemapId(tilemap),
                    ..Default::default()
                },
                RigidBody::Static,
                Collider::rectangle(40.0, 40.0),
                Transform::from_xyz(world.x, world.y, 0.0),
            ))
            .id(),
        _ => commands
            .spawn(TileBundle {
                position: tile_pos,
                tilemap_id: TilemapId(tilemap),
                ..Default::default()
            })
            .id(),
    }
}

fn create_tilemap(
    commands: &mut Commands,
    map_size: TilemapSize,
    image: Handle<Image>,
    typ: MapType,
    z: f32,
    depth: i32,
    physics: bool,
) -> Entity {
    let tilemap_entity = commands.spawn_empty().id();

    let tile_size = TilemapTileSize { x: 20.0, y: 20.0 };

    let mut tile_storage = TileStorage::empty(map_size);
    for x in 0..map_size.x {
        for y in 0..map_size.y {
            let tile_pos = TilePos { x, y };
            let tile_entity = spawn_tile(
                commands,
                tilemap_entity,
                typ,
                map_size,
                tile_size,
                tile_pos,
                physics,
            );
            tile_storage.set(&tile_pos, tile_entity);
        }
    }
    let grid_size = tile_size.into();
    let map_type = TilemapType::default();

    let mut map = HashMap::new();
    map.insert(0b0011, vec![TileTextureIndex(0), TileTextureIndex(6)]);
    map.insert(0b0111, vec![TileTextureIndex(2), TileTextureIndex(7)]);
    map.insert(0b0101, vec![TileTextureIndex(4), TileTextureIndex(8)]);
    map.insert(
        0b1011,
        vec![
            TileTextureIndex(10),
            TileTextureIndex(11),
            TileTextureIndex(15),
        ],
    );
    map.insert(0b1111, vec![TileTextureIndex(12)]);
    map.insert(0b1101, vec![TileTextureIndex(13)]);
    map.insert(0b1010, vec![TileTextureIndex(16)]);
    map.insert(0b1110, vec![TileTextureIndex(17)]);
    map.insert(0b1100, vec![TileTextureIndex(18)]);
    map.insert(0b0010, vec![TileTextureIndex(21)]);
    map.insert(0b0110, vec![TileTextureIndex(22), TileTextureIndex(23)]);
    map.insert(0b0100, vec![TileTextureIndex(24)]);

    let bundle = (
        TilemapBundle {
            grid_size,
            map_type,
            size: map_size,
            storage: tile_storage,
            texture: TilemapTexture::Single(image),
            tile_size,
            anchor: TilemapAnchor::Center,
            transform: Transform::from_scale(
                Vec3::new(2.0, 2.0, 1.0) * (1.0 + (depth as f32) / 420.0),
            )
            .with_translation(Vec3::new(0.0, 0.0, z)),
            render_settings: TilemapRenderSettings {
                render_chunk_size: UVec2::new(32, 32),
                y_sort: false,
            },
            ..Default::default()
        },
        BitMap { map },
        MapDepth(depth),
        typ,
    );
    commands.entity(tilemap_entity).insert(bundle);
    tilemap_entity
}

pub fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let texture_handle: Handle<Image> = asset_server.load("tilemap.png");
    let background_handle: Handle<Image> = asset_server.load("tilemap_background1.png");
    let background_handl2: Handle<Image> = asset_server.load("tilemap_background2.png");
    let size = TilemapSize { x: 80, y: 50 };
    let front = create_tilemap(
        &mut commands,
        size,
        texture_handle.clone(),
        MapType::Command,
        0.0,
        1,
        true,
    );
    commands.insert_resource(PhysicsTilemap(front));

    create_tilemap(
        &mut commands,
        size,
        background_handle.clone(),
        MapType::Companion(front),
        -1.0,
        -1,
        false,
    );
    create_tilemap(
        &mut commands,
        size,
        background_handle.clone(),
        MapType::Companion(front),
        -3.0,
        -3,
        false,
    );
    create_tilemap(
        &mut commands,
        size,
        background_handle.clone(),
        MapType::Companion(front),
        -5.0,
        -5,
        false,
    );
    create_tilemap(
        &mut commands,
        size,
        background_handle.clone(),
        MapType::Companion(front),
        -7.0,
        -7,
        false,
    );
    create_tilemap(
        &mut commands,
        size,
        background_handle.clone(),
        MapType::Companion(front),
        -9.0,
        -9,
        false,
    );
    create_tilemap(
        &mut commands,
        size,
        background_handl2.clone(),
        MapType::Companion(front),
        -1.0,
        -9,
        false,
    );

    let middle = create_tilemap(
        &mut commands,
        size,
        texture_handle.clone(),
        MapType::Command,
        -10.0,
        -10,
        false,
    );

    create_tilemap(
        &mut commands,
        size,
        background_handle.clone(),
        MapType::Companion(middle),
        -12.0,
        -12,
        false,
    );
    create_tilemap(
        &mut commands,
        size,
        background_handle.clone(),
        MapType::Companion(middle),
        -14.0,
        -14,
        false,
    );

    commands.insert_resource(TilemapGroups(vec![front, middle]));
}

pub fn bitmap(
    tilemap_query: Query<(&TileStorage, &TilemapSize, &BitMap, &MapType)>,
    mut tile_query: Query<&mut TileTextureIndex>,
) {
    for (storage, size, bitmap, map_type) in tilemap_query {
        for x in 0..size.x {
            for y in 0..size.y {
                let this = TilePos { x, y };
                let Some(this_entity) = storage.get(&this) else {
                    continue;
                };

                match map_type {
                    MapType::Command => {
                        let neighbors = Neighbors::get_square_neighboring_positions(
                            &TilePos { x, y },
                            size,
                            false,
                        );
                        let mut id = 0u8;
                        for neighbor in neighbors.iter() {
                            if storage.get(neighbor).is_none() {
                                continue;
                            }

                            id |= TO_BITMAP
                                .get(&(
                                    neighbor.x as i32 - this.x as i32,
                                    neighbor.y as i32 - this.y as i32,
                                ))
                                .unwrap();
                        }

                        if let Ok(mut this_texture) = tile_query.get_mut(this_entity) {
                            let default = vec![TileTextureIndex(6)];
                            let tiles = bitmap.map.get(&id).unwrap_or(&default);
                            *this_texture = *tiles
                                .get(if tiles.len() > 1 {
                                    rand::rng().random_range(0..tiles.len())
                                } else {
                                    0
                                })
                                .unwrap();
                        }
                    }
                    MapType::Companion(entity) => {
                        if let Ok((storage, _, _, _)) = tilemap_query.get(*entity) {
                            if let Some(command_id) = storage.get(&this) {
                                if let other_texture =
                                    *tile_query.get(command_id).unwrap_or(&TileTextureIndex(6))
                                    && let Ok(mut this_texture) = tile_query.get_mut(this_entity)
                                {
                                    *this_texture = other_texture;
                                }
                            } else if let Ok(mut this_texture) = tile_query.get_mut(this_entity) {
                                *this_texture = TileTextureIndex(1);
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn depth(
    camera_query: Query<&mut GlobalTransform, (With<Camera2d>, Without<BitMap>)>,
    tilemap_query: Query<(&mut GlobalTransform, &MapDepth), With<BitMap>>,
) {
    let Ok(camera_transform) = camera_query.single() else {
        return;
    };
    let camera_pos = camera_transform.translation().xy();
    for (mut global_transform, &MapDepth(depth)) in tilemap_query {
        *global_transform = Transform::from_xyz(
            camera_pos.x * (-depth as f32 / 180.0),
            camera_pos.y * (-depth as f32 / 180.0),
            global_transform.translation().z,
        )
        .with_scale(global_transform.scale())
        .into();
    }
}

pub fn colordepth(mut q_tiles: Query<&mut TileColor>, q_tilemap: Query<(&TileStorage, &MapDepth)>) {
    for (storage, depth) in q_tilemap {
        let depth = depth.0 as f32;

        for entity in storage.iter() {
            let Some(entity) = entity else {
                continue;
            };

            let mut color = q_tiles.get_mut(*entity).unwrap();
            let amount = 1.0 - depth / 50.0;
            *color = TileColor(Color::srgb(amount, amount * 0.2, amount * 0.2));
        }
    }
}

pub fn edit(
    mut commands: Commands,
    mouse_input: Res<ButtonInput<MouseButton>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    tilemap_groups: Res<TilemapGroups>,
    tilemap_query: Query<
        (
            Entity,
            &mut TileStorage,
            &TilemapSize,
            &TilemapGridSize,
            &TilemapTileSize,
            &TilemapType,
            &TilemapAnchor,
            &GlobalTransform,
            &MapType,
        ),
        Without<MainCamera>,
    >,
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

    let index = if keyboard_input.pressed(KeyCode::ControlLeft) {
        1
    } else if keyboard_input.pressed(KeyCode::ShiftLeft) {
        2
    } else {
        0
    };
    let Some(group) = tilemap_groups.0.get(index) else {
        return;
    };

    for (
        tilemap_id,
        mut storage,
        map_size,
        grid_size,
        tile_size,
        map_type,
        anchor,
        transform,
        typ,
    ) in tilemap_query
    {
        if let MapType::Companion(_) = typ {
            continue;
        }

        if group != &tilemap_id {
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
            }
            if mouse_input.pressed(MouseButton::Left)
                && let None = storage.get(&position)
            {
                let tile_entity = spawn_tile(
                    &mut commands,
                    tilemap_id,
                    *typ,
                    *map_size,
                    *tile_size,
                    position,
                    if index == 0 { true } else { false },
                );
                storage.set(&position, tile_entity);
            }
        }
    }
}

pub fn update_current_tile(
    query: Query<(&mut CurrentTile, &Transform)>,
    tilemap_query: Query<(
        &TilemapSize,
        &TilemapGridSize,
        &TilemapTileSize,
        &TilemapType,
        &TilemapAnchor,
    )>,
) {
    for (mut current_tile, transform) in query {
        let (map_size, grid_size, tile_size, map_type, anchor) =
            tilemap_query.get(current_tile.tilemap).unwrap();
        current_tile.tile = Some(
            TilePos::from_world_pos(
                &(transform.translation.xy() / 2.0),
                map_size,
                grid_size,
                tile_size,
                map_type,
                anchor,
            )
            .unwrap(),
        );
    }
}
