use bevy::{
    asset::RenderAssetUsages,
    math::{NormedVectorSpace, VectorSpace},
    prelude::*,
    render::render_resource::{AsBindGroup, Extent3d, ShaderType, TextureDimension, TextureFormat},
    sprite_render::Material2d,
    tasks::Task,
};
use bevy_ecs_tilemap::prelude::*;

use crate::tilemap::{BitMap, MapDepth, MapType};

pub struct LightingLayer {
    pub atlas: Image,
    pub tiles: Vec<Option<UVec2>>,
    pub depth: i32,
    pub scale: f32,
    pub tile_size: f32,
    pub map_size: UVec2,
    pub atlas_size: UVec2,
}

pub struct LightingData {
    pub layers: Vec<LightingLayer>,
    pub size: UVec2,
}

pub struct Heightmap {
    pub image: Image,
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
    pub heightmap: Handle<Image>,

    #[uniform(2)]
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

    let mut image = Image::new(
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

    let half = Vec2::new(data.size.x as f32 / 2.0, data.size.y as f32 / 2.0);

    for x in 0..data.size.x {
        for y in 0..data.size.y {
            let pos = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);

            for layer in &data.layers {
                let relative = (pos - half) / (layer.scale / 2.0) + half;
                let grid = (relative / layer.tile_size).floor().as_uvec2();
                let index = grid.y * layer.map_size.x + grid.x;
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

                    let colordepth = 1.0 + (layer.depth as f32) / 20.0;
                    let already = image.get_color_at(x, data.size.y - y - 1).unwrap();
                    if colordepth < already.to_srgba().red {
                        continue;
                    }

                    image
                        .set_color_at(x, data.size.y - y - 1, Color::srgb(colordepth, 0.0, 0.0))
                        .unwrap();
                }
            }
        }
    }

    Heightmap { image }
}

pub fn save_heightmap(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<HeightmapMaterial>>,
    asset_server: Res<AssetServer>,
    images: Res<Assets<Image>>,
    q_tilemap: Query<(
        &TileStorage,
        &BitMap,
        &MapDepth,
        &Transform,
        &TilemapTileSize,
        &TilemapSize,
    )>,
    q_tiles: Query<&TileTextureIndex>,
) {
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
            atlas_size: bitmap.dimensions * 20,
        });
    }

    let data = LightingData {
        layers,
        size: UVec2::new(80 * 20, 50 * 20),
    };

    let heightmap = construct_heightmap(data);
    let dynamic = heightmap.image.clone().try_into_dynamic().unwrap();
    dynamic.save("heightmap.png").unwrap();

    let handle = asset_server.add(heightmap.image);
    let settings = ShadowSettings {
        light_dir: Vec3::new(-1.0, -0.5, 0.8),
        height_scale: 20.0,
        shadow_color: Vec4::new(0.0, 0.0, 0.0, 0.5),
        step_size: 0.001,
        max_steps: 30,
        _padding: Vec2::default(),
    };
    let material = HeightmapMaterial {
        heightmap: handle,
        settings,
    };

    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(80.0 * 20.0, 50.0 * 20.0))),
        MeshMaterial2d(materials.add(material)),
        Transform::from_scale(Vec3::splat(2.0)),
        Visibility::default(),
    ));
}
