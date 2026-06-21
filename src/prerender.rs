use std::collections::HashSet;

use bevy::prelude::*;

#[derive(Component, Debug, Clone)]
struct Atlas {
    image: Handle<Image>,
    dimensions: UVec2,
    tile_dimensions: Vec2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum TileType {}

#[derive(Component, Debug, Clone)]
struct TileBehavior(HashSet<TileType>);

#[derive(Debug, Clone)]
struct Element {
    image: Handle<Image>,
    offset: Vec2,
}

#[derive(Component, Debug, Clone)]
struct Elements(Vec<Element>);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Effect {}

#[derive(Component, Debug, Clone)]
struct Effects(HashSet<Effect>);

#[derive(Component, Debug, Clone)]
struct Tile {
    atlas: Entity,
    atlas_pos: UVec2,
    tile_pos: UVec2,
    dimensions: Vec2,
}

#[derive(Component, Debug, Clone)]
struct Layer {
    tiles: Vec<Option<Entity>>,
    dimensions: UVec2,
    layer: i32,
    output: Handle<Image>,
}

struct Level {
    layers: Vec<Layer>,
    output: Handle<Image>,
}
