use std::{collections::HashMap, f32};

use bevy::prelude::*;
use bevy_spatial::SpatialAccess;

use crate::{MainCamera, PIXEL_SCALE};

#[derive(Resource)]
pub struct BranchAssets {
    pub mesh: Handle<Mesh>,
    pub material: Handle<ColorMaterial>,
}

#[derive(Component, Debug, Clone)]
pub struct Params {
    /// Distance at which attractors affect a branch.
    pub attraction: f32,
    /// Distance at which a branch consumes an attractor.
    pub kill: f32,
    /// Maximum length a branch can be.
    pub max_length: f32,
    /// Width of the highest level branches.
    pub base_width: f32,
    /// Growth rate in pixels per second, multiplied by the number of
    /// attractors.
    pub growth: f32,
    pub exponent: f32,
    /// Percentage that a branch's direction can vary from it's parent.
    pub variance: f32,
    pub min_vigor: f32,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct Root;

#[derive(Component, Debug, Clone, Copy)]
pub struct Tip;

#[derive(Component, Debug, Clone, Copy)]
pub struct Final;

#[derive(Component, Debug, Clone)]
pub struct Branch {
    pub parent_direction: Option<Vec2>,
    pub offshoots: Vec<Entity>,
    pub pos: Vec2,
    pub direction: Option<Vec2>,
    pub length: f32,
    pub width: f32,
    pub vigor: f32,
    pub params: Entity,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct Attractor {
    pub pos: Vec2,
}

pub fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.insert_resource(BranchAssets {
        mesh: meshes.add(Rectangle::default()),
        material: materials.add(Color::WHITE),
    });
}

pub fn clamp_direction(target_dir: Vec2, reference_dir: Vec2, max_angle_radians: f32) -> Vec2 {
    let target = target_dir.normalize_or_zero();
    let reference = reference_dir.normalize_or_zero();

    let angle = reference.angle_to(target);

    if angle.abs() <= max_angle_radians {
        return target;
    }

    let clamped_angle = angle.clamp(-max_angle_radians, max_angle_radians);

    let (sin, cos) = clamped_angle.sin_cos();

    Vec2::new(
        reference.x * cos - reference.y * sin,
        reference.x * sin + reference.y * cos,
    )
}

pub fn grow(
    mut commands: Commands,
    time: Res<Time>,
    tree: Res<bevy_spatial::kdtree::KDTree2<Attractor>>,
    assets: Res<BranchAssets>,
    q_params: Query<&Params>,
    q_branch: Query<(Entity, &mut Branch), Without<Final>>,
    q_tip: Query<&Tip>,
) {
    let mut attractors = Vec::new();
    let mut distances = HashMap::new();

    for (entity, mut branch) in q_branch {
        let params = q_params.get(branch.params).unwrap();

        for (pos, entity) in tree.within_distance(branch.pos, params.attraction) {
            let Some(entity) = entity else {
                continue;
            };
            let distance_sq = branch.pos.distance_squared(pos);
            if let Some(other) = distances.get(&entity)
                && distance_sq > *other
            {
                continue;
            }
            if distance_sq < params.attraction * params.attraction {
                attractors.push(pos);
                distances.insert(entity, distance_sq);
            }
        }

        if branch.length < params.max_length {
            if branch.vigor > params.min_vigor {
                branch.length +=
                    params.growth * attractors.len() as f32 * time.delta_secs() * branch.vigor;
            }
        } else if branch.offshoots.len() < 2 {
            let split_id = commands
                .spawn((
                    Tip,
                    Branch {
                        parent_direction: Some(branch.direction.unwrap()),
                        offshoots: Vec::new(),
                        pos: branch.pos,
                        direction: None,
                        length: 0.0,
                        width: params.base_width,
                        params: branch.params,
                        vigor: branch.vigor
                            * (branch.direction.unwrap().dot(Vec2::Y) / 4.0 + 0.75).max(0.1),
                    },
                    Mesh2d(assets.mesh.clone()),
                    MeshMaterial2d(assets.material.clone()),
                    Transform::default(),
                ))
                .id();
            branch.offshoots.push(split_id);

            if q_tip.get(entity).is_ok() {
                commands.entity(entity).remove::<Tip>();
            }

            if branch.offshoots.len() == 2 {
                commands.get_entity(entity).unwrap().insert(Final);
            }
        }

        if branch.direction.is_none() && !attractors.is_empty() {
            let target = attractors.iter().sum::<Vec2>() / attractors.len() as f32;
            let gravity = Vec2::new(0.0, (-0.2 * (1.0 - branch.width / 50.0)).max(0.0));
            branch.direction = Some((target - branch.pos).normalize() + gravity);
            if let Some(parent) = branch.parent_direction {
                branch.direction = Some(clamp_direction(
                    branch.direction.unwrap(),
                    parent,
                    params.variance * f32::consts::PI,
                ));
            }
        }

        attractors.clear();
    }
}

pub fn kill(
    mut commands: Commands,
    tree: Res<bevy_spatial::kdtree::KDTree2<Attractor>>,
    q_params: Query<&Params>,
    q_branch: Query<&Branch>,
) {
    for branch in q_branch {
        let Some(direction) = branch.direction else {
            continue;
        };

        let params = q_params.get(branch.params).unwrap();

        for (pos, entity) in tree.within_distance(branch.pos, params.attraction) {
            let Some(entity) = entity else {
                continue;
            };
            if (branch.pos + direction * branch.length).distance(pos) < params.kill {
                commands.entity(entity).despawn();
            }
        }
    }
}

pub fn position(
    q_root: Query<&Branch, With<Root>>,
    mut q_branch: Query<&mut Branch, Without<Root>>,
) {
    for root in q_root {
        position_root(root, &mut q_branch);
    }
}

fn position_root(root: &Branch, q_branch: &mut Query<&mut Branch, Without<Root>>) {
    for entity in &root.offshoots {
        let Ok(mut offshoot) = q_branch.get_mut(*entity) else {
            continue;
        };
        offshoot.pos = root.pos + root.direction.unwrap() * root.length;
        position_root(&offshoot.clone(), q_branch);
    }
}

pub fn width(q_params: Query<&Params>, mut q_branches: Query<(Entity, &mut Branch)>) {
    let mut new_widths = Vec::with_capacity(q_branches.iter().count());

    for (entity, branch) in q_branches.iter() {
        let params = q_params.get(branch.params).unwrap();

        let area_sum = branch
            .offshoots
            .iter()
            .map(|child_entity| {
                if let Ok((_, child_branch)) = q_branches.get(*child_entity) {
                    child_branch.width.powf(params.exponent)
                } else {
                    0.0
                }
            })
            .sum::<f32>();

        if area_sum > 0.0 {
            let new_width = area_sum.powf(1.0 / params.exponent);
            new_widths.push((entity, new_width));
        }
    }

    for (entity, width) in new_widths {
        if let Ok((_, mut branch)) = q_branches.get_mut(entity) {
            branch.width = width;
        }
    }
}

pub fn render(q_branch: Query<(&Branch, &mut Transform)>) {
    for (branch, mut transform) in q_branch {
        if let Some(direction) = branch.direction {
            let pos = branch.pos + direction * branch.length / 2.0;
            *transform = Transform::from_translation(Vec3::new(pos.x, pos.y, 0.0))
                .with_scale(Vec3::new(branch.length, branch.width, 1.0))
                .with_rotation(Quat::from_rotation_z(direction.to_angle()));
        }
    }
}

pub fn debug_plants(
    mut gizmos: Gizmos,
    q_roots: Query<&Branch, (With<Root>, Without<Tip>)>,
    q_tips: Query<&Branch, (Without<Root>, Without<Final>)>,
) {
    for root in q_roots {
        gizmos.circle_2d(root.pos, 5.0, Color::srgb(1.0, 0.5, 0.2));
    }

    for tip in q_tips {
        gizmos.circle_2d(tip.pos, 5.0, Color::srgb(0.2, 1.0, 0.5));
    }
}

pub fn debug_attractors(mut gizmos: Gizmos, q_attractor: Query<&Attractor>) {
    for attractor in q_attractor {
        gizmos.circle_2d(attractor.pos, 5.0, Color::srgb(0.8, 0.2, 0.5));
    }
}

pub fn spawn_attractor(
    mut commands: Commands,
    window_query: Query<&Window>,
    camera_query: Query<(&Camera, &mut GlobalTransform), With<MainCamera>>,
) {
    let Ok(window) = window_query.single() else {
        return;
    };
    let Ok((camera, camera_transform)) = camera_query.single() else {
        return;
    };
    let Some(cursor) = window.cursor_position().and_then(|cursor| {
        camera
            .viewport_to_world_2d(camera_transform, cursor / PIXEL_SCALE)
            .ok()
    }) else {
        return;
    };

    for _ in 0..10 {
        let pos = cursor
            + Vec2::new(
                rand::random_range(-20.0..20.0),
                rand::random_range(-20.0..20.0),
            );
        commands.spawn((Attractor { pos }, Transform::from_xyz(pos.x, pos.y, 0.0)));
    }
}

pub fn spawn_root(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    window_query: Query<&Window>,
    camera_query: Query<(&Camera, &mut GlobalTransform), With<MainCamera>>,
) {
    let Ok(window) = window_query.single() else {
        return;
    };
    let Ok((camera, camera_transform)) = camera_query.single() else {
        return;
    };
    let Some(cursor) = window.cursor_position().and_then(|cursor| {
        camera
            .viewport_to_world_2d(camera_transform, cursor / PIXEL_SCALE)
            .ok()
    }) else {
        return;
    };

    let params = commands
        .spawn(Params {
            attraction: 80.0,
            kill: 5.0,
            max_length: 7.0,
            base_width: 1.5,
            growth: 3.0,
            exponent: 2.8,
            variance: 0.25,
            min_vigor: 0.1,
        })
        .id();

    commands.spawn((
        Root,
        Tip,
        Branch {
            parent_direction: None,
            offshoots: Vec::new(),
            pos: cursor,
            direction: None,
            length: 0.0,
            width: 2.0,
            vigor: 1.0,
            params,
        },
        Mesh2d(meshes.add(Rectangle::default())),
        MeshMaterial2d(materials.add(Color::WHITE)),
        Transform::default(),
    ));
}
