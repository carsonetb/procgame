use avian2d::prelude::*;
use bevy::prelude::*;

use crate::health::Damages;

#[derive(Component, Default, Debug, Clone, Copy)]
pub struct Bullet {
    pub speed: Vec2,
}

#[derive(Resource, Debug, Clone)]
pub struct NormalBulletData {
    mesh: Handle<Mesh>,
    material: Handle<ColorMaterial>,
}

pub fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.insert_resource(NormalBulletData {
        mesh: meshes.add(Circle::new(2.0)),
        material: materials.add(Color::srgb(1.0, 1.0, 1.0)),
    });
}

/// Really fast moving rock
pub fn spawn_normal(commands: &mut Commands, data: &Res<NormalBulletData>, pos: Vec2, speed: Vec2) {
    commands.spawn((
        Bullet { speed },
        Damages {
            damage: 1.0,
            speed_multi: 1.0,
            cooldown: Timer::from_seconds(0.0, TimerMode::Once),
        },
        Collider::circle(2.0),
        Mesh2d(data.mesh.clone()),
        MeshMaterial2d(data.material.clone()),
        Transform::from_xyz(pos.x, pos.y, 0.0),
    ));
}

pub fn gravity(time: Res<Time>, q_bullet: Query<&mut Bullet>) {
    for mut bullet in q_bullet {
        bullet.speed.y -= 980.0 * time.delta_secs();
    }
}

pub fn travel(time: Res<Time>, q_bullet: Query<(&Bullet, &mut Transform)>) {
    for (bullet, mut transform) in q_bullet {
        transform.translation += (bullet.speed * time.delta_secs()).extend(0.0);
    }
}
