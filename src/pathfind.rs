use std::collections::HashSet;

use bevy::prelude::*;
use bevy_ecs_tilemap::{helpers::square_grid::neighbors::Neighbors, prelude::*};

#[derive(Component, Debug, Clone)]
pub struct Pathfinding {
    pub from: Vec2,
    pub to: Vec2,
    pub steps: i32,
    pub direction: Option<Vec2>,
    pub tilemap: Entity,
}

impl Pathfinding {
    pub fn new(tilemap: Entity, from: Vec2, to: Vec2, steps: i32) -> Self {
        Self {
            from,
            to,
            steps,
            direction: None,
            tilemap,
        }
    }
}

fn viable(storage: &TileStorage, pos: TilePos) -> bool {
    !(storage.get(&TilePos::new(pos.x - 1, pos.y)).is_none()
        && storage.get(&TilePos::new(pos.x + 1, pos.y)).is_none()
        && storage.get(&TilePos::new(pos.x, pos.y - 1)).is_none()
        && storage.get(&TilePos::new(pos.x - 1, pos.y - 1)).is_none()
        && storage.get(&TilePos::new(pos.x + 1, pos.y - 1)).is_none())
}

pub fn pathfind(
    // mut gizmos: Gizmos,
    q_pathfinding: Query<&mut Pathfinding>,
    q_tilemap: Query<(
        &mut TileStorage,
        &TilemapSize,
        &TilemapGridSize,
        &TilemapTileSize,
        &TilemapType,
        &TilemapAnchor,
    )>,
) {
    for mut pathfinding in q_pathfinding {
        let (storage, map_size, grid_size, tile_size, map_type, anchor) =
            q_tilemap.get(pathfinding.tilemap).unwrap();
        let mut explored = HashSet::new();

        let mut origin = TilePos::from_world_pos(
            &(pathfinding.from / 2.0),
            map_size,
            grid_size,
            tile_size,
            map_type,
            anchor,
        )
        .unwrap();

        while !viable(storage, origin) {
            origin.y -= 1;
        }

        let mut unexplored: Vec<TilePos> = vec![origin];

        for _ in 0..pathfinding.steps {
            let mut new_unexplored = HashSet::new();
            for unexplored in unexplored {
                if !viable(storage, unexplored) {
                    continue;
                }

                explored.insert(unexplored);

                for neighbor in
                    Neighbors::get_square_neighboring_positions(&unexplored, map_size, false).iter()
                {
                    if explored.contains(neighbor) || storage.get(neighbor).is_some() {
                        continue;
                    }

                    new_unexplored.insert(*neighbor);
                }
            }
            unexplored = new_unexplored.into_iter().collect();
        }

        let mut closest_pos =
            origin.center_in_world(map_size, grid_size, tile_size, map_type, anchor);
        let mut closest_distance = f32::INFINITY;
        for pos in explored {
            let pos = pos.center_in_world(map_size, grid_size, tile_size, map_type, anchor) * 2.0;
            // gizmos.circle_2d(pos, 5.0, Color::srgb(0.0, 0.0, 1.0));
            let distance = pos.distance(pathfinding.to);
            if distance < closest_distance {
                closest_distance = distance;
                closest_pos = pos;
            }
        }

        pathfinding.direction = Some((closest_pos - pathfinding.from).normalize());
    }
}
