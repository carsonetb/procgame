use std::f32::consts::PI;

use bevy::prelude::*;
use rand::RngExt;

#[derive(Debug, Clone, Copy)]
pub struct Action<C> {
    pub command: C,
    pub weight: f32,
}

#[derive(Debug, Clone)]
pub struct Sense<S: Clone> {
    pub direction: f32,
    pub distance: f32,
    pub direction_precision: f32,
    pub distance_precision: f32,
    pub sense_type: S,
}

impl<S: Clone> Sense<S> {
    pub fn new(
        direction: f32,
        distance: f32,
        direction_precision: f32,
        distance_precision: f32,
        sense_type: S,
    ) -> Self {
        Self {
            direction,
            distance,
            direction_precision,
            distance_precision,
            sense_type,
        }
    }

    pub fn internal(sense: S) -> Self {
        Self::new(1.0, 1.0, 1.0, 1.0, sense)
    }

    pub fn get_pos(&self, rng: &mut rand::rngs::StdRng) -> (f32, f32, Vec2) {
        let distance = self.distance * self.distance_precision.powf(rng.random_range(-1.0..=1.0));

        let diroffset = (1.0 - self.direction_precision) * PI;
        let direction =
            rng.random_range((self.direction - diroffset)..(self.direction + diroffset));
        let vector = Vec2::from_angle(direction) * distance;
        (distance, direction, vector)
    }
}

pub trait Belief<S: Clone, C>: Send + Sync {
    fn decay(&mut self, delta: f32);
    fn update(&mut self, sense: Sense<S>);
    fn minimize(&mut self) -> Vec<C>;
}

pub trait SmartEntity<C> {
    fn apply(&mut self, commands: Vec<C>, delta: f32);
}

#[derive(Component)]
pub struct Brain<S, C> {
    pub states: Vec<Box<dyn Belief<S, C>>>,
}

impl<S: Clone, C> Brain<S, C> {
    pub fn new(states: Vec<Box<dyn Belief<S, C>>>) -> Self {
        Self { states }
    }

    pub fn apply<E: SmartEntity<C>>(&mut self, entity: &mut E, senses: Vec<Sense<S>>, delta: f32) {
        let mut commands = Vec::new();
        for state in &mut self.states {
            state.decay(delta);
            for sense in &senses {
                state.update(sense.clone());
            }
            commands.append(&mut state.minimize());
        }

        entity.apply(commands, delta);
    }
}
