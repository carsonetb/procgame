use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::{AsBindGroup, Extent3d, ShaderType, TextureDimension, TextureFormat},
    sprite_render::Material2d,
    tasks::{AsyncComputeTaskPool, Task, block_on, poll_once},
};
use bevy_ecs_tilemap::prelude::*;

use crate::{
    plants::Branch,
    tilemap::{BitMap, MapDepth},
};

pub struct LightingLayer {
    pub atlas: Image,
    pub tiles: Vec<Option<UVec2>>,
    pub plants: Vec<Transform>,
    pub depth: i32,
    pub scale: f32,
    pub tile_size: f32,
    pub map_size: UVec2,
}

pub struct LightingData {
    pub layers: Vec<LightingLayer>,
    pub size: UVec2,
}

pub struct Heightmap {
    pub front: Image,
    pub back: Image,
}

#[derive(Debug, Clone, ShaderType)]
pub struct ShadowSettings {
    pub light_dir: Vec3,
    pub height_scale: f32,
    pub shadow_color: Vec4,
    pub step_size: f32,
    pub max_steps: i32,
    pub _padding: Vec2,
}

#[derive(Debug, Clone, Asset, TypePath, AsBindGroup)]
pub struct HeightmapMaterial {
    #[texture(0)]
    #[sampler(1)]
    pub front: Handle<Image>,

    #[texture(2)]
    #[sampler(3)]
    pub back: Handle<Image>,

    #[uniform(4)]
    pub settings: ShadowSettings,
}

impl Material2d for HeightmapMaterial {
    fn fragment_shader() -> bevy::shader::ShaderRef {
        "shaders/shadow.wgsl".into()
    }

    fn alpha_mode(&self) -> bevy::sprite_render::AlphaMode2d {
        bevy::sprite_render::AlphaMode2d::Blend
    }
}

#[derive(Component)]
pub struct LightingTask(Task<Heightmap>);

pub fn construct_heightmap(mut data: LightingData) -> Heightmap {
    data.layers.sort_by(|l, r| l.depth.cmp(&r.depth));
    data.layers.reverse();
    // Sorted from closest to camera to furthest from camera.

    let mut front = Image::new(
        Extent3d {
            width: data.size.x,
            height: data.size.y,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        vec![0; (data.size.x * data.size.y * 4) as usize],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );

    let mut back = Image::new(
        Extent3d {
            width: data.size.x,
            height: data.size.y,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        vec![255; (data.size.x * data.size.y * 4) as usize],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );

    let half = Vec2::new(data.size.x as f32 / 2.0, data.size.y as f32 / 2.0);

    for x in 0..data.size.x {
        for y in 0..data.size.y {
            let pos = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);

            for layer in &data.layers {
                let colordepth = 1.0 + (layer.depth as f32) / 20.0;
                let front_already = front.get_color_at(x, data.size.y - y - 1).unwrap();
                let back_already = back.get_color_at(x, data.size.y - y - 1).unwrap();
                if colordepth < front_already.to_srgba().red
                    && colordepth > back_already.to_srgba().red
                {
                    continue;
                }

                let relative = (pos - half) / (layer.scale / 2.0) + half;
                let grid = (relative / layer.tile_size).floor().as_uvec2();
                let index = grid.y * layer.map_size.x + grid.x;

                // for transform in &layer.plants {
                //     let world = (pos - half) * 2.0;
                //     let inverse = transform.compute_affine().inverse();
                //     let local = inverse.transform_point3(world.extend(0.0));
                //     if local.length() < 1.0 {
                //         front_image
                //             .set_color_at(x, data.size.y - y - 1, Color::srgb(colordepth, 0.0, 0.0))
                //             .unwrap();

                //         break;
                //     }
                // }

                if let Some(Some(atlas_point)) = layer.tiles.get(index as usize) {
                    let local_relative = relative - (grid.as_vec2() * layer.tile_size);
                    let pixel_in_tile = local_relative.floor().as_uvec2();

                    let max_px = (layer.tile_size - 1.0) as u32;
                    let point = atlas_point
                        + UVec2::new(
                            pixel_in_tile.x.min(max_px),
                            max_px - pixel_in_tile.y.min(max_px),
                        );

                    let tile_color = layer.atlas.get_color_at(point.x, point.y).unwrap();
                    if tile_color.to_srgba().alpha < 0.05 {
                        continue;
                    }

                    if colordepth > front_already.to_srgba().red {
                        front
                            .set_color_at(x, data.size.y - y - 1, Color::srgb(colordepth, 0.0, 0.0))
                            .unwrap();
                    }

                    if colordepth < back_already.to_srgba().red {
                        back.set_color_at(
                            x,
                            data.size.y - y - 1,
                            Color::srgb(colordepth, 0.0, 0.0),
                        )
                        .unwrap();
                    }
                }
            }
        }
    }

    Heightmap { front, back }
}

pub fn trigger_heightmap_work(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    images: Res<Assets<Image>>,
    q_plants: Query<&Transform, With<Branch>>,
    q_tilemap: Query<(
        &TileStorage,
        &BitMap,
        &MapDepth,
        &Transform,
        &TilemapTileSize,
        &TilemapSize,
    )>,
    q_tiles: Query<&TileTextureIndex>,
    q_lighting: Query<Entity, With<MeshMaterial2d<HeightmapMaterial>>>,
) {
    for entity in q_lighting {
        commands.entity(entity).despawn();
    }

    let task = save_heightmap(asset_server, images, q_plants, q_tilemap, q_tiles);

    commands.spawn(LightingTask(task));
}

pub fn poll_heightmap_work(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<HeightmapMaterial>>,
    asset_server: Res<AssetServer>,
    tasks: Query<(Entity, &mut LightingTask)>,
) {
    for (entity, mut task) in tasks {
        if let Some(heightmap) = block_on(poll_once(&mut task.0)) {
            let dynamic = heightmap.front.clone().try_into_dynamic().unwrap();
            dynamic.save("heightmap_front.png").unwrap();

            let dynamic = heightmap.back.clone().try_into_dynamic().unwrap();
            dynamic.save("heightmap_back.png").unwrap();

            let front = asset_server.add(heightmap.front);
            let back = asset_server.add(heightmap.back);
            let settings = ShadowSettings {
                light_dir: Vec3::new(-1.0, -0.5, 0.8),
                height_scale: 20.0,
                shadow_color: Vec4::new(0.0, 0.0, 0.0, 0.5),
                step_size: 0.001,
                max_steps: 30,
                _padding: Vec2::default(),
            };
            let material = HeightmapMaterial {
                front,
                back,
                settings,
            };

            commands.spawn((
                Mesh2d(meshes.add(Rectangle::new(80.0 * 20.0, 50.0 * 20.0))),
                MeshMaterial2d(materials.add(material)),
                Transform::from_scale(Vec3::splat(2.0)),
                Visibility::default(),
            ));

            commands.entity(entity).despawn();
        }
    }
}

pub fn save_heightmap(
    asset_server: Res<AssetServer>,
    images: Res<Assets<Image>>,
    q_plants: Query<&Transform, With<Branch>>,
    q_tilemap: Query<(
        &TileStorage,
        &BitMap,
        &MapDepth,
        &Transform,
        &TilemapTileSize,
        &TilemapSize,
    )>,
    q_tiles: Query<&TileTextureIndex>,
) -> Task<Heightmap> {
    let mut layers = Vec::new();

    for (storage, bitmap, depth, transform, tile_size, tilemap_size) in q_tilemap {
        let depth = depth.0;
        let scale = transform.scale.x;
        let tile_size = tile_size.x;
        let map_size = UVec2::new(tilemap_size.x, tilemap_size.y);
        let atlas = asset_server.load(&bitmap.texture);
        let atlas = images.get(&atlas).unwrap().clone();

        let mut tiles = Vec::new();
        for tile in storage.iter() {
            match tile {
                Some(entity) => {
                    let texture = q_tiles.get(*entity).unwrap();
                    let pos = UVec2::new(
                        (texture.0 % bitmap.dimensions.x) * 20, // TODO: Magic number
                        (texture.0 / bitmap.dimensions.x) * 20,
                    );
                    tiles.push(Some(pos));
                }
                None => tiles.push(None),
            }
        }

        layers.push(LightingLayer {
            atlas,
            tiles,
            depth,
            scale,
            tile_size,
            map_size,
            plants: if depth == 1 {
                q_plants.into_iter().cloned().collect::<Vec<_>>()
            } else {
                Vec::new()
            },
        });
    }

    let data = LightingData {
        layers,
        size: UVec2::new(80 * 20, 50 * 20),
    };

    let thread_pool = AsyncComputeTaskPool::get();
    thread_pool.spawn(async move { construct_heightmap(data) })
}
