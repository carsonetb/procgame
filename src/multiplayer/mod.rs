//! Setting this aside for a little bit to do couch multiplayer instead.

use std::collections::HashMap;

use bevy::prelude::*;

pub use local::*;
use serde::{Deserialize, Serialize};

mod local;

#[derive(Component, Debug, Clone, Copy)]
pub struct NetworkedEntity {
    pub authority: bool,
    pub id: u64,
}

#[derive(Component, Serialize, Deserialize, Debug, Deref, Clone)]
pub struct Synchronized<T> {
    id: u64,
    #[deref]
    inner: T,
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

        #[derive(Message, Deref, Debug, Clone)]
        pub struct SendMessage(Message);

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
                                let Some(network_info) = world.get::<NetworkedEntity>(local_entity).cloned() else {
                                    warn!("Entity lacks Authority component.");
                                    return;
                                };

                                let Some(mut local) = world.get_mut::<Synchronized<$comp>>(local_entity) else {
                                    warn!("Entity lacks component {}.", stringify!($comp));
                                    return;
                                };

                                if network_info.authority {
                                    error!("I am authority but am receiving updates for {}.", stringify!($comp));
                                    return;
                                }

                                local.inner = remote.inner;
                            });
                        }
                    )*
                }
            }
        }

        pub fn send_messages(
            world: &mut World
        ) {
            $(
                let mut query = world.query::<(&Synchronized<$comp>, &NetworkedEntity)>();
                let iter = query.iter_mut(world);
                let mut messages = Vec::with_capacity(iter.len());
                for (local, info) in iter {
                    if !info.authority {
                        continue;
                    }
                    info!("Sending a message from {}", stringify!($comp));
                    let mut local = local.clone();
                    local.id = info.id;
                    messages.push(SendMessage(Message::$comp(local)))
                }

                for message in messages {
                    world.write_message(message);
                }
            )*
        }
    };
}

define_messages! {
    Transform,
}
