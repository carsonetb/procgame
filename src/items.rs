use bevy::{prelude::*, sprite_render::AlphaMode2d};

use crate::instance::Instance;

#[derive(Component, Default, Debug, Clone, Copy)]
pub struct Item;

#[derive(Component, Default, Debug, Clone)]
pub struct HoldPoints {
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

pub fn build_stick(commands: &mut Commands, assets: &Res<ItemAssets>, pos: Vec2) -> Instance<Item> {
    let id = commands
        .spawn((
            Item,
            HoldPoints {
                primary: Vec2::new(-5.0, 0.0),
                secondary: Some(Vec2::new(5.0, 0.0)),
            },
            Transform::from_xyz(pos.x, pos.y, 200.0).with_scale(Vec3::new(30.0, 3.0, 1.0)),
            Mesh2d(assets.stick_mesh.clone()),
            MeshMaterial2d(assets.stick_material.clone()),
            Visibility::default(),
        ))
        .id();
    Instance::from(id)
}
