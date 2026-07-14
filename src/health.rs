use avian2d::prelude::*;
use bevy::prelude::*;

#[derive(Component, Default, Debug, Clone)]
pub struct Damages {
    /// Damage done if the object is traveling at 100 pixels per second.
    pub damage: f32,
    /// Modifies damage by speed_multi * ((speed - 100.0) / 100.0)
    pub speed_multi: f32,
    pub cooldown: Timer,
}

#[derive(Component, Default, Debug, Clone, Copy)]
pub struct Health {
    pub health: f32,
}

pub fn cooldown(time: Res<Time>, q_damages: Query<&mut Damages>) {
    for mut damages in q_damages {
        damages.cooldown.tick(time.delta());
    }
}

pub fn damage(q_health: Query<(&mut Health, &CollidingEntities)>, q_damages: Query<&Damages>) {
    for (mut health, colliding) in q_health {
        for entity in colliding.iter() {
            if let Ok(damages) = q_damages.get(*entity)
                && damages.cooldown.is_finished()
            {
                health.health -= damages.damage;
            }
        }
    }
}
