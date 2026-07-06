use bevy::prelude::*;

#[derive(Component, Debug, Clone)]
pub struct Params {
    /// Distance at which attractors affect a branch.
    pub attraction: f32,
    /// Distance at which a branch consumes an attractor.
    pub kill: f32,
    /// Maximum length a branch can be.
    pub max_length: f32,
    // pub branch_length: f32,
    /// Width of the highest level branches.
    pub base_width: f32,
    /// Growth rate in pixels per second, multiplied by the number of
    /// attractors.
    pub growth: f32,
    pub exponent: f32,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct Root;

#[derive(Component, Debug, Clone, Copy)]
pub struct Tip;

#[derive(Component, Debug, Clone)]
pub struct Branch {
    pub begin: Option<Entity>,
    pub offshoots: Vec<Entity>,
    pub pos: Vec2,
    pub direction: Option<Vec2>,
    pub length: f32,
    pub width: f32,
    pub params: Entity,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct Attractor {
    pub pos: Vec2,
}

pub fn grow(
    time: Res<Time>,
    q_params: Query<&Params>,
    q_branch: Query<&mut Branch>,
    q_attractor: Query<&Attractor>,
) {
    let mut attractors = Vec::new();

    for mut branch in q_branch {
        let params = q_params.get(branch.params).unwrap();

        for attractor in q_attractor {
            if branch.pos.distance(attractor.pos) < params.attraction {
                attractors.push(attractor.pos);
            }
        }

        branch.length += params.growth * attractors.len() as f32 * time.delta_secs();

        if branch.direction.is_none() {
            branch.direction = Some(attractors.iter().sum::<Vec2>() / attractors.len() as f32)
        }

        attractors.clear();
    }
}

pub fn kill(
    mut commands: Commands,
    q_params: Query<&Params>,
    q_branch: Query<(Entity, &mut Branch)>,
    q_attractor: Query<(Entity, &Attractor)>,
) {
    for (entity, mut branch) in q_branch {
        let params = q_params.get(branch.params).unwrap();

        let mut split = false;
        for (entity, attractor) in q_attractor {
            if branch.pos.distance(attractor.pos) < params.kill {
                commands.entity(entity).despawn();
                split = true;
            }
        }

        if split {
            let split_id = commands
                .spawn(Branch {
                    begin: Some(entity),
                    offshoots: Vec::new(),
                    pos: branch.pos,
                    direction: None,
                    length: 0.0,
                    width: params.base_width,
                    params: branch.params,
                })
                .id();
            branch.offshoots.push(split_id);
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
        let mut offshoot = q_branch.get_mut(*entity).unwrap();
        offshoot.pos = root.pos + root.direction.unwrap() * root.length;
        position_root(&offshoot.clone(), q_branch);
    }
}

pub fn width(q_params: Query<&Params>, mut q_branch: Query<(Entity, &mut Branch), Without<Tip>>) {
    let mut widths = Vec::with_capacity(q_branch.count());
    for (entity, branch) in &q_branch {
        let params = q_params.get(branch.params).unwrap();
        let width = branch
            .offshoots
            .iter()
            .map(|branch| q_branch.get(*branch).unwrap().1.width.powf(params.exponent))
            .sum::<f32>()
            .powf(1.0 / params.exponent);
        widths.push((entity, width));
    }

    for (entity, width) in widths {
        q_branch.get_mut(entity).unwrap().1.width = width;
    }
}
