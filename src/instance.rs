use std::marker::PhantomData;

use bevy::prelude::*;

#[derive(Debug)]
pub struct Instance<T: Component> {
    pub entity: Entity,
    phantom: PhantomData<T>,
}

impl<T: Component> Clone for Instance<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Component> Copy for Instance<T> {}

impl<T: Component> From<Entity> for Instance<T> {
    fn from(entity: Entity) -> Self {
        Self {
            entity,
            phantom: PhantomData,
        }
    }
}
