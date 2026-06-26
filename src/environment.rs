use std::collections::HashSet;

use bevy::prelude::*;
use bevy_ecs_tilemap::{helpers::square_grid::neighbors::Neighbors, prelude::*};

use crate::tilemap::PhysicsTilemap;

#[derive(Debug, Clone, Copy)]
pub enum AuditoryEventType {
    Walk,
    Scuttle,
}

#[derive(Debug, Clone, Copy)]
pub enum VisualEventType {
    Food { size: f32, color: Color },
}

#[derive(Debug, Clone, Copy)]
pub enum InternalEventType {
    Hunger { intensity: f32 },
    Movement { amount: Vec2 },
    Position(Vec2),
    Tile(TilePos, Vec2),
}

#[derive(Debug, Clone)]
pub enum EventType {
    Auditory {
        typ: AuditoryEventType,
        affects: HashSet<TilePos>,
    },
    Visual(VisualEventType),
    Internal(InternalEventType),
}

#[derive(Component, Debug, Clone)]
pub struct SenseEvent {
    pub event_type: EventType,
    pub position: Vec2,
    pub intensity: f32,
    decay_speed: f32,
}

#[derive(Component, Debug, Clone)]
pub struct Emitter {
    pub event: SenseEvent,
    pub interval: Timer,
}

impl SenseEvent {
    pub fn new(typ: EventType, position: Vec2, intensity: f32, decay_speed: f32) -> Self {
        Self {
            event_type: typ,
            position,
            intensity,
            decay_speed,
        }
    }

    pub fn internal(typ: EventType) -> Self {
        Self::new(typ, Vec2::new(0.0, 0.0), 0.0, 0.0)
    }
}

pub fn process(time: Res<Time>, query: Query<(Entity, &mut SenseEvent)>, mut commands: Commands) {
    for (entity, mut event) in query {
        event.intensity -= event.decay_speed * time.delta_secs();
        if event.intensity <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
}

pub fn sound(
    physics_tilemap: Res<PhysicsTilemap>,
    mut q_senses: Query<&mut SenseEvent>,
    q_tilemap: Query<(&TileStorage, &TilemapSize)>,
) {
    let (storage, map_size) = q_tilemap.get(physics_tilemap.0).unwrap();

    for _ in 0..2 {
        for mut sense in q_senses.iter_mut() {
            let EventType::Auditory { affects, .. } = &mut sense.event_type else {
                continue;
            };

            for affect in affects.clone() {
                for neighbor in
                    Neighbors::get_square_neighboring_positions(&affect, map_size, false).iter()
                {
                    if affects.contains(&neighbor) || storage.get(&neighbor).is_some() {
                        continue;
                    }

                    affects.insert(*neighbor);
                }
            }
        }
    }
}

pub fn debug_sound(
    mut gizmos: Gizmos,
    q_senses: Query<&mut SenseEvent>,
    physics_tilemap: Res<PhysicsTilemap>,
    q_tilemap: Query<(
        &TilemapSize,
        &TilemapGridSize,
        &TilemapTileSize,
        &TilemapType,
        &TilemapAnchor,
    )>,
) {
    let (map_size, grid_size, tile_size, map_type, anchor) =
        q_tilemap.get(physics_tilemap.0).unwrap();

    for mut sense in q_senses {
        let EventType::Auditory { affects, .. } = &mut sense.event_type else {
            continue;
        };

        for affect in affects.clone() {
            let pos =
                affect.center_in_world(map_size, grid_size, tile_size, map_type, anchor) * 2.0;
            gizmos.circle_2d(pos, 5.0, Color::srgb(1.0, 1.0, 0.0));
        }
    }
}

pub fn emit(
    mut commands: Commands,
    time: Res<Time>,
    q_emitter: Query<(&mut Emitter, &Transform)>,
    physics_tilemap: Res<PhysicsTilemap>,
    q_tilemap: Query<(
        &TilemapSize,
        &TilemapGridSize,
        &TilemapTileSize,
        &TilemapType,
        &TilemapAnchor,
    )>,
) {
    let (map_size, grid_size, tile_size, map_type, anchor) =
        q_tilemap.get(physics_tilemap.0).unwrap();

    for (mut emitter, transform) in q_emitter {
        let pos = transform.translation.xy();

        emitter.interval.tick(time.delta());
        if emitter.interval.just_finished() {
            let mut new = emitter.event.clone();

            if let EventType::Auditory { affects, .. } = &mut new.event_type {
                *affects = HashSet::new();
                affects.insert(
                    TilePos::from_world_pos(
                        &(pos / 2.0),
                        map_size,
                        grid_size,
                        tile_size,
                        map_type,
                        anchor,
                    )
                    .unwrap(),
                );
            }

            new.position = pos;
            commands.spawn(new);
        }
    }
}
