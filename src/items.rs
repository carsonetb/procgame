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
    pub offset: Vec2,
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
    q_item: Query<(&Item, &mut HoldPoints, &mut Transform)>,
    q_player: Query<&Transform, (With<Player>, With<BodyHead>, Without<Item>)>,
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

    let Some(player_transform) = q_player.iter().next() else {
        return;
    };
    let player_pos = player_transform.translation.xy();

    for (item, mut holds, mut transform) in q_item {
        if !item.held {
            continue;
        }

        transform.rotation = Quat::from_rotation_z((cursor - player_pos).to_angle());
        let cursor_direction = (cursor + Vec2::new(0.01, 0.01) - player_pos).normalize();

        let multiplier = ((cursor - player_pos) / 20.0).length().min(30.0);
        holds.primary = cursor_direction * (4.0 + multiplier);
        if let Some(secondary) = &mut holds.secondary {
            *secondary = cursor_direction * (multiplier - 9.0);
        }
        holds.offset = cursor_direction * multiplier;
    }
}

pub fn update_interactions(mut commands: Commands, q_items: Query<(Entity, &Item, &RigidBody)>) {
    for (entity, item, body) in q_items {
        if item.held
            && let RigidBody::Dynamic = body
        {
            commands.entity(entity).insert(RigidBody::Kinematic);
        }
        if !item.held
            && let RigidBody::Kinematic = body
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
                offset: Vec2::default(),
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
