use std::{collections::HashMap, f32};

use bevy::{
    math::FloatPow,
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task, block_on, poll_once},
};
use bevy_spatial::{SpatialAccess, kdtree::KDTree2};

use crate::{MainCamera, PIXEL_SCALE, instance::Instance};

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
    pub branches: i32,
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
    pub params: Instance<Params>,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct Attractor {
    pub pos: Vec2,
}

struct AddedBranch {
    branch: Branch,
    from: usize,
}

pub struct PlantData {
    params: Params,
    attractors: bevy_spatial::kdtree::KDTree2<Attractor>,
    branches: Vec<(Entity, Branch)>,
    added: Vec<AddedBranch>,
    killed: Vec<Instance<Attractor>>,
}

#[derive(Component)]
pub struct PlantsTask(Task<PlantData>);

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

pub fn grow_task(mut data: PlantData) -> PlantData {
    let mut attractor_owners = HashMap::new();

    for (i, (_, branch)) in data.branches.iter().enumerate() {
        let is_finished = branch.length >= data.params.max_length
            && branch.offshoots.len() >= data.params.branches as usize;
        if is_finished {
            continue;
        }

        // Branches grow from their tip, so measure distance from the tip!
        let tip_pos = if let Some(dir) = branch.direction {
            branch.pos + dir * branch.length
        } else {
            branch.pos
        };

        let within = data
            .attractors
            .within_distance(tip_pos, data.params.attraction);

        for (pos, entity) in within {
            let Some(entity) = entity else {
                continue;
            };

            let distance_sq = tip_pos.distance_squared(pos);

            if distance_sq < data.params.attraction.squared() {
                let entry = attractor_owners.entry(entity).or_insert((i, f32::MAX, pos));
                // Only overwrite if this branch is strictly closer
                if distance_sq < entry.1 {
                    *entry = (i, distance_sq, pos);
                }
            }
        }
    }

    let mut branch_attractors: HashMap<usize, Vec<Vec2>> = HashMap::new();
    for (_, (branch_idx, _, pos)) in attractor_owners {
        branch_attractors.entry(branch_idx).or_default().push(pos);
    }

    for (i, (_, branch)) in data.branches.iter_mut().enumerate() {
        let attractors = branch_attractors
            .get(&i)
            .map(|v| v.as_slice())
            .unwrap_or(&[]);

        if branch.length < data.params.max_length {
            if branch.vigor > data.params.min_vigor {
                branch.length += data.params.growth * attractors.len() as f32 * 0.01 * branch.vigor;
            }
        } else if branch.offshoots.len() < data.params.branches as usize {
            data.added.push(AddedBranch {
                branch: Branch {
                    parent_direction: Some(branch.direction.unwrap()),
                    offshoots: Vec::new(),
                    pos: branch.pos + branch.direction.unwrap() * branch.length,
                    direction: None,
                    length: 0.0,
                    width: data.params.base_width,
                    params: branch.params,
                    vigor: branch.vigor
                        * (branch.direction.unwrap().dot(Vec2::Y) / 4.0 + 0.75).max(0.1),
                },
                from: i,
            });
        }

        if branch.direction.is_none() && !attractors.is_empty() {
            let target = attractors.iter().sum::<Vec2>() / attractors.len() as f32;
            let tropism = Vec2::Y * (0.5 * branch.vigor);
            let gravity = Vec2::new(0.0, (-0.2 * (1.0 - data.params.base_width / 50.0)).max(0.0));
            let mut direction =
                Some(((target - branch.pos).normalize() + tropism + gravity).normalize());

            if let Some(parent) = branch.parent_direction {
                let stiffness = branch.vigor.clamp(0.0, 1.0);
                let dynamic_variance = data.params.variance * (1.0 - (stiffness * 0.75));
                direction = Some(clamp_direction(
                    direction.unwrap(),
                    parent,
                    dynamic_variance * std::f32::consts::PI,
                ));
            }
            branch.direction = direction;
        }
    }

    for (_, branch) in &data.branches {
        let Some(direction) = branch.direction else {
            continue;
        };

        let within = data.attractors.within_distance(
            branch.pos + direction * branch.length,
            data.params.attraction,
        );
        for (pos, entity) in within {
            let Some(entity) = entity else {
                continue;
            };

            if (branch.pos + direction * branch.length).distance(pos) < data.params.kill {
                data.killed.push(Instance::from(entity));
            }
        }
    }

    data
}

pub fn poll_grow_task(
    mut commands: Commands,
    assets: Res<BranchAssets>,
    q_task: Query<(Entity, &mut PlantsTask)>,
    q_tip: Query<&Tip>,
    mut q_branch: Query<&mut Branch>,
) {
    for (entity, mut task) in q_task {
        let Some(mut data) = block_on(poll_once(&mut task.0)) else {
            continue;
        };

        for (entity, branch) in &data.branches {
            let mut live = q_branch.get_mut(*entity).unwrap();
            live.length = branch.length;
            live.direction = branch.direction;
        }

        for attractor in data.killed.drain(..) {
            if let Ok(mut entity) = commands.get_entity(attractor.entity) {
                entity.despawn();
            }
        }

        let mut finals = Vec::new();
        for branch in data.added.drain(..) {
            let entity = commands
                .spawn((
                    Tip,
                    branch.branch.clone(),
                    Mesh2d(assets.mesh.clone()),
                    MeshMaterial2d(assets.material.clone()),
                    Transform::default(),
                    Visibility::default(),
                ))
                .id();
            data.branches.push((entity, branch.branch));

            let (from_entity, from) = data.branches.get_mut(branch.from).unwrap();
            from.offshoots.push(entity);
            q_branch
                .get_mut(*from_entity)
                .unwrap()
                .offshoots
                .push(entity);

            if q_tip.get(*from_entity).is_ok() {
                commands.entity(*from_entity).remove::<Tip>();
            }

            if from.offshoots.len() == data.params.branches as usize {
                commands.get_entity(*from_entity).unwrap().insert(Final);
                finals.push(*from_entity);
            }
        }

        for entity in finals {
            if let Some(remove) = data.branches.iter().rposition(|(e, _)| e == &entity) {
                data.branches.remove(remove);
            }
        }

        commands.entity(entity).despawn();

        let thread_pool = AsyncComputeTaskPool::get();
        let task = thread_pool.spawn(async move { grow_task(data) });
        commands.spawn(PlantsTask(task));
    }
}

pub fn kill(
    commands: ParallelCommands,
    tree: Res<bevy_spatial::kdtree::KDTree2<Attractor>>,
    q_params: Query<&Params>,
    q_branch: Query<&Branch, With<Tip>>,
) {
    q_branch.par_iter().for_each(|branch| {
        let Some(direction) = branch.direction else {
            return;
        };

        let params = q_params.get(branch.params.entity).unwrap();

        for (pos, entity) in
            tree.within_distance(branch.pos + direction * branch.length, params.attraction)
        {
            let Some(entity) = entity else {
                continue;
            };
            commands.command_scope(|mut commands| {
                if (branch.pos + direction * branch.length).distance(pos) < params.kill
                    && let Ok(mut entity) = commands.get_entity(entity)
                {
                    entity.despawn();
                }
            })
        }
    });
}

pub fn width(q_params: Query<&Params>, mut q_branches: Query<(Entity, &mut Branch), With<Final>>) {
    let mut new_widths = Vec::with_capacity(q_branches.iter().count());

    for (entity, branch) in q_branches.iter() {
        let params = q_params.get(branch.params.entity).unwrap();

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
                .with_scale(Vec3::new(branch.length, branch.width.max(1.5), 1.0))
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
    attractors: Res<bevy_spatial::kdtree::KDTree2<Attractor>>,
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

    let params = Params {
        attraction: 35.0,
        kill: 5.0,
        max_length: 4.0,
        base_width: 1.0,
        growth: 10.0,
        exponent: 2.8,
        variance: 0.15,
        min_vigor: 0.05,
        branches: 2,
    };

    let params_entity = commands.spawn(params.clone()).id();

    let branch = Branch {
        parent_direction: None,
        offshoots: Vec::new(),
        pos: cursor,
        direction: Some(Vec2::Y),
        length: 0.0,
        width: 1.0,
        vigor: 1.0,
        params: Instance::from(params_entity),
    };
    let branch_entity = commands
        .spawn((
            Root,
            Tip,
            branch.clone(),
            Mesh2d(meshes.add(Rectangle::default())),
            MeshMaterial2d(materials.add(Color::WHITE)),
            Transform::default(),
        ))
        .id();

    let mut tree = KDTree2::default();
    tree.tree = attractors.tree.clone();
    let data = PlantData {
        params,
        attractors: tree,
        branches: vec![(branch_entity, branch)],
        added: Vec::new(),
        killed: Vec::new(),
    };

    let thread_pool = AsyncComputeTaskPool::get();
    let task = thread_pool.spawn(async move { grow_task(data) });
    commands.spawn(PlantsTask(task));
}
