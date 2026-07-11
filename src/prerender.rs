use std::collections::{HashMap, HashSet};

use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use bitflags::bitflags;
use phf_macros::phf_map;

#[derive(Debug, Clone)]
pub struct Matrix<T: Copy> {
    pub width: u32,
    pub height: u32,
    pub values: Vec<Vec<T>>,
}

impl<T: Copy> Matrix<T> {
    pub fn new(width: u32, height: u32, default: T) -> Self {
        let mut values = Vec::with_capacity(width as usize);
        for _ in 0..width {
            let mut column = Vec::with_capacity(height as usize);
            for _ in 0..height {
                column.push(default);
            }
            values.push(column);
        }
        Self {
            width,
            height,
            values,
        }
    }

    pub fn set(&mut self, x: u32, y: u32, value: T) {
        self.values[x as usize][y as usize] = value;
    }

    pub fn get(&self, x: u32, y: u32) -> Option<T> {
        self.values.get(x as usize)?.get(y as usize).cloned()
    }

    pub fn get_mut(&mut self, x: u32, y: u32) -> Option<&mut T> {
        self.values.get_mut(x as usize)?.get_mut(y as usize)
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct BitDirection: u8 {
        const EMPTY = 0b0000;
        const UP = 0b1000;
        const LEFT = 0b0100;
        const RIGHT = 0b0010;
        const DOWN = 0b0001;
    }
}

const TO_BIT: phf::Map<(i32, i32), BitDirection> = phf_map! {
    (0, 1) =>  BitDirection::UP,
    (-1, 0) =>  BitDirection::LEFT,
    (1, 0) =>   BitDirection::RIGHT,
    (0, -1) =>   BitDirection::DOWN,
};

#[derive(Debug, Clone)]
pub struct Atlas {
    pub image: Handle<Image>,
    pub dimensions: UVec2,
    pub tile_dimensions: UVec2,
    pub bitmap: HashMap<BitDirection, UVec2>,
    pub fallback: UVec2,
}

#[derive(Resource, Debug, Clone)]
pub struct AtlasDB {
    atlasses: Vec<Atlas>,
}

impl AtlasDB {
    pub fn new() -> Self {
        Self {
            atlasses: Vec::new(),
        }
    }

    pub fn total(&self) -> usize {
        self.atlasses.len()
    }

    pub fn add(&mut self, atlas: Atlas) -> AtlasID {
        self.atlasses.push(atlas);
        AtlasID(self.atlasses.len() - 1)
    }

    pub fn get(&self, id: AtlasID) -> Option<&Atlas> {
        self.atlasses.get(id.0)
    }

    pub fn get_mut(&mut self, id: AtlasID) -> Option<&mut Atlas> {
        self.atlasses.get_mut(id.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AtlasID(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum TileType {}

#[derive(Component, Debug, Clone)]
pub struct TileBehavior(HashSet<TileType>);

#[derive(Debug, Clone)]
struct Element {
    image: Handle<Image>,
    offset: Vec2,
}

#[derive(Component, Debug, Clone)]
pub struct Elements(Vec<Element>);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Effect {}

#[derive(Component, Debug, Clone)]
pub struct Effects(HashSet<Effect>);

#[derive(Component, Debug, Clone)]
pub struct Tile {
    pub atlas: AtlasID,
    pub layer: Entity,
    pub atlas_pos: UVec2,
    pub tile_pos: UVec2,
}

#[derive(Component, Debug, Clone)]
pub struct Layer {
    pub tiles: Matrix<Option<Entity>>,
    pub dimensions: UVec2,
    pub layer: i32,
    pub owner: i32,
    pub output: Matrix<Color>,
    pub following: bool,
}

#[derive(Resource, Debug, Clone)]
pub struct Level {
    output: Matrix<Color>,
    image: Handle<Image>,
}

pub fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let image = Image::new_fill(
        Extent3d {
            width: 40 * 20,
            height: 40 * 20,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &Srgba::new(0.0, 0.0, 0.0, 1.0).to_u8_array(),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    let image = images.add(image);

    commands.insert_resource(AtlasDB::new());
    commands.insert_resource(Level {
        output: Matrix::new(40 * 20, 40 * 20, Color::BLACK),
        image,
    });
}

pub fn clean(mut level: ResMut<Level>, layers: Query<&mut Layer>) {
    println!("-- Beginning render process --");

    level.output = Matrix::new(level.output.width, level.output.height, Color::NONE);

    for mut layer in layers {
        layer.output = Matrix::new(layer.output.width, layer.output.height, Color::NONE);
    }
}

pub fn bitmap_tile(
    atlas_db: Res<AtlasDB>,
    tile_query: Query<&mut Tile>,
    layer_query: Query<&Layer>,
) {
    for mut tile in tile_query {
        let Ok(layer) = layer_query.get(tile.layer) else {
            return error!("A tile has a nonexistant layer.");
        };
        let Some(atlas) = atlas_db.get(tile.atlas) else {
            return error!("A tile has a nonexistant atlas.");
        };

        const NEIGHBORS: [IVec2; 4] = [
            IVec2::new(-1, 0),
            IVec2::new(1, 0),
            IVec2::new(0, -1),
            IVec2::new(0, 1),
        ];

        let mut bit_id = BitDirection::EMPTY;
        for neighbor in NEIGHBORS {
            let pos = tile.tile_pos.as_ivec2() + neighbor;
            if pos.x < 0
                || pos.x > layer.dimensions.x as i32
                || pos.y < 0
                || pos.y > layer.dimensions.y as i32
            {
                continue;
            }

            if let Some(tile) = layer.tiles.get(pos.x as u32, pos.y as u32)
                && tile.is_some()
            {
                bit_id |= TO_BIT[&(neighbor.x, neighbor.y)];
            }
        }

        if let Some(atlas_pos) = atlas.bitmap.get(&bit_id) {
            tile.atlas_pos = *atlas_pos;
        } else {
            tile.atlas_pos = atlas.fallback;
        }
    }
}

pub fn render_tile(
    images: ResMut<Assets<Image>>,
    atlas_db: Res<AtlasDB>,
    tile_query: Query<(&Tile, Option<&Effects>, Option<&Elements>)>,
    mut layer_query: Query<&mut Layer>,
) {
    println!("Rendering tiles...");
    for (tile, effects, elements) in tile_query {
        if let Some(atlas) = atlas_db.get(tile.atlas)
            && let Ok(mut layer) = layer_query.get_mut(tile.layer)
            && let Some(atlas_image) = images.get(&atlas.image)
        {
            let atlas_pos = tile.atlas_pos * atlas.tile_dimensions;
            let world_pos = UVec2::new(tile.tile_pos.x, layer.dimensions.y - tile.tile_pos.y - 1)
                * atlas.tile_dimensions;

            for x in 0..atlas.tile_dimensions.x {
                for y in 0..atlas.tile_dimensions.y {
                    let tile_color = atlas_image
                        .get_color_at(atlas_pos.x + x, atlas_pos.y + y)
                        .unwrap();
                    layer
                        .output
                        .set(world_pos.x + x, world_pos.y + y, tile_color);
                }
            }
        }
    }
}

fn blend_colors(base: Color, overlay: Color) -> Color {
    let base = LinearRgba::from(base);
    let overlay = LinearRgba::from(overlay);

    Color::from(LinearRgba::new(
        base.red.lerp(overlay.red, overlay.alpha),
        base.green.lerp(overlay.green, overlay.alpha),
        base.blue.lerp(overlay.blue, overlay.alpha),
        base.alpha.max(overlay.alpha),
    ))
}

pub fn render_level(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut level: ResMut<Level>,
    layer_query: Query<&Layer>,
) {
    let mut layers = layer_query.iter().collect::<Vec<_>>();
    layers.sort_by(|a, b| a.layer.cmp(&b.layer));

    println!("Rendering layers...");

    for layer in layers {
        assert_eq!(level.output.width, layer.output.width);
        assert_eq!(level.output.height, layer.output.height);

        for x in 0..layer.output.width {
            for y in 0..layer.output.height {
                let in_color = layer.output.get(x, y).unwrap();
                let out_color = level.output.get(x, y).unwrap();
                level.output.set(x, y, blend_colors(out_color, in_color));
            }
        }
    }

    let mut image = images.get_mut(&level.image).unwrap();
    assert_eq!(level.output.width, image.width());
    assert_eq!(level.output.height, image.height());

    println!("Rendering to image...");

    for x in 0..level.output.width {
        for y in 0..level.output.height {
            image
                .set_color_at(x, y, level.output.get(x, y).unwrap())
                .unwrap();
        }
    }

    commands.spawn((
        RenderExample,
        Sprite::from_image(level.image.clone()),
        Transform::from_xyz(0.0, 0.0, 1.0).with_scale(Vec3::splat(1.5)),
    ));

    let Ok(image) = image.clone().try_into_dynamic() else {
        return error!("Could not convert image to save.");
    };

    println!("Saving...");

    image.save("level_output.png").unwrap();

    println!("Done");
}

#[derive(Component, Debug, Clone, Copy)]
pub struct RenderExample;

pub fn remove_example(mut commands: Commands, query: Query<Entity, With<RenderExample>>) {
    for entity in query {
        commands.entity(entity).despawn();
    }
}
