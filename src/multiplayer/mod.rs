use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

use bevy::prelude::*;

pub use local::*;
use serde::{Deserialize, Serialize};

mod local;

#[derive(Component, Serialize, Deserialize, Debug, Clone)]
pub struct Synchronized<T> {
    id: u64,
    authority: bool,
    inner: T,
}

impl<T> Deref for Synchronized<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> DerefMut for Synchronized<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

#[derive(Resource, Debug, Default, Clone)]
pub struct Translator {
    map: HashMap<u64, Entity>,
}

macro_rules! define_messages {
    ($($comp:ident),* $(,)?) => {
        #[derive(Message, Serialize, Deserialize, Debug, Clone)]
        pub enum Message {
            $(
                $comp(Synchronized<$comp>),
            )*
        }

        #[allow(non_snake_case)]
        pub fn process_messages(
            mut messages: MessageReader<Message>,
            translator: Res<Translator>,
            mut commands: Commands,
        ) {
            for message in messages.read() {
                match message {
                    $(
                        Message::$comp(remote) => {
                            let Some(&local_entity) = translator.map.get(&remote.id) else {
                                warn!("Received {} for unknown network ID.", stringify!($comp));
                                continue;
                            };

                            let remote = remote.clone();

                            commands.queue(move |world: &mut World| {
                                let Some(mut local) = world.get_mut::<Synchronized<$comp>>(local_entity) else {
                                    warn!("Entity lacks component: {}", stringify!($comp));
                                    return;
                                };

                                if local.authority || !remote.authority {
                                    error!("Authority mismatch for {}.", stringify!($comp));
                                    return;
                                }

                                local.inner = remote.inner;
                            });
                        }
                    )*
                }
            }
        }
    };
}

define_messages! {
    Transform,
}
