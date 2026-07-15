use avian2d::prelude::*;
use bevy::{prelude::*, sprite_render::AlphaMode2d};

use crate::{
    GameLayer, MainCamera, PIXEL_SCALE, body::BodyHead, instance::Instance, player::Player,
};

#[derive(Component, Default, Debug, Clone, Copy)]
pub struct Item {
    pub held: bool,
}

#[derive(Component, Default, Debug, Clone)]
pub struct HoldPoints {
    pub base_primary: Vec2,
    pub base_secondary: Option<Vec2>,
    pub primary: Vec2,
    pub secondary: Option<Vec2>,
}

#[derive(Resource, Debug, Clone)]
pub struct ItemAssets {
    pub stick_mesh: Handle<Mesh>,
    pub stick_material: Handle<ColorMaterial>,
}

pub fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.insert_resource(ItemAssets {
        stick_mesh: meshes.add(Rectangle::default()),
        stick_material: materials.add(ColorMaterial {
            color: Color::WHITE,
            alpha_mode: AlphaMode2d::Blend,
            ..default()
        }),
    });
}

pub fn towards_mouse(
    q_window: Query<&Window>,
    q_camera: Query<(&Camera, &mut GlobalTransform), With<MainCamera>>,
    q_item: Query<(&Item, &mut HoldPoints)>,
    q_player: Query<&Transform, (With<Player>, With<BodyHead>)>,
) {
    let Ok(window) = q_window.single() else {
        return;
    };
    let Ok((camera, camera_transform)) = q_camera.single() else {
        return;
    };
    let Some(cursor) = window.cursor_position().and_then(|cursor| {
        camera
            .viewport_to_world_2d(camera_transform, cursor / PIXEL_SCALE)
            .ok()
    }) else {
        return;
    };

    let Ok(player_transform) = q_player.single() else {
        return;
    };
    let player_pos = player_transform.translation.xy();

    for (item, mut holds) in q_item {
        if !item.held {
            continue;
        }

        let mut primary_target = holds.base_primary + (cursor - player_pos) / 10.0;
        if primary_target.length() > 30.0 {
            primary_target = primary_target.normalize() * 30.0;
        }
        holds.primary = holds.primary.lerp(primary_target, 0.1);
        if let Some(base_secondary) = holds.base_secondary
            && let Some(secondary) = &mut holds.secondary
        {
            let mut secondary_target = base_secondary + (cursor - player_pos) / 10.0;
            if secondary_target.length() > 30.0 {
                secondary_target = secondary_target.normalize() * 30.0;
            }
            *secondary = secondary.lerp(secondary_target, 0.1);
        }
    }
}

pub fn update_interactions(mut commands: Commands, q_items: Query<(Entity, &Item, &RigidBody)>) {
    for (entity, item, body) in q_items {
        if item.held
            && let RigidBody::Dynamic = body
        {
            commands.entity(entity).insert(RigidBody::Static);
        }
        if !item.held
            && let RigidBody::Static = body
        {
            commands.entity(entity).insert(RigidBody::Dynamic);
        }
    }
}

pub fn build_stick(commands: &mut Commands, assets: &Res<ItemAssets>, pos: Vec2) -> Instance<Item> {
    let id = commands
        .spawn((
            Item { held: true },
            HoldPoints {
                base_primary: Vec2::new(-5.0, 0.0),
                base_secondary: Some(Vec2::new(5.0, 0.0)),
                primary: Vec2::new(-5.0, 0.0),
                secondary: Some(Vec2::new(5.0, 0.0)),
            },
            RigidBody::Dynamic,
            Collider::rectangle(1.0, 1.0),
            CollisionLayers::new(GameLayer::Items, [GameLayer::Environment]),
            Transform::from_xyz(pos.x, pos.y, 200.0).with_scale(Vec3::new(30.0, 3.0, 1.0)),
            Mesh2d(assets.stick_mesh.clone()),
            MeshMaterial2d(assets.stick_material.clone()),
            Visibility::default(),
        ))
        .id();
    Instance::from(id)
}
