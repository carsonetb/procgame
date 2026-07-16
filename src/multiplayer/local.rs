use std::{
    marker::PhantomData,
    net::SocketAddr,
    sync::{Mutex, mpsc},
    thread,
};

use bevy::prelude::*;
use message_io::{
    network::{Endpoint, NetEvent, ToRemoteAddr, Transport},
    node::{self, NodeHandler},
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

pub struct LocalNetworkingPlugin<T> {
    pub port: String,
    pub target_port: Option<String>,
    phantom: PhantomData<T>,
}

impl<T> LocalNetworkingPlugin<T> {
    pub fn new(port: String, target: Option<String>) -> Self {
        Self {
            port,
            target_port: target,
            phantom: PhantomData,
        }
    }
}

impl<T: Serialize + DeserializeOwned + Send + Sync + 'static> Plugin for LocalNetworkingPlugin<T> {
    fn build(&self, app: &mut App) {
        let (handler, listener) = node::split::<()>();

        let listen = format!("0.0.0.0:{}", self.port);
        let (resource, _) = handler.network().listen(Transport::Udp, &listen).unwrap();
        info!("Listening on {}", listen);

        let mut endpoints = Vec::new();
        let mut clients = Vec::new();
        if let Some(target) = self.target_port.clone() {
            let target = format!("127.0.0.1:{}", target);
            info!("Greeting {}", target);
            let endpoint =
                Endpoint::from_listener(resource, *target.to_remote_addr().unwrap().socket_addr());
            endpoints.push(endpoint);
            clients.push(target);

            handler.network().send(
                endpoint,
                &bincode::serialize(&NetworkMessage::<T>::Handshake).unwrap(),
            );
        }

        let (sender, receiver) = mpsc::channel::<(Endpoint, Vec<u8>)>();

        thread::spawn(move || {
            listener.for_each(move |event| match event.network() {
                NetEvent::Message(endpoint, data) => {
                    let _ = sender.send((endpoint, data.to_vec()));
                }
                NetEvent::Disconnected(endpoint) => {
                    info!("{} disconnected", endpoint.addr());
                }
                _ => (),
            });
        });

        app.insert_resource(Networking {
            receiver: Mutex::new(receiver),
            handler,
            clients: endpoints.iter().map(|endpoint| endpoint.addr()).collect(),
            authority: self.target_port.is_none(),
        });

        app.add_systems(FixedUpdate, update::<T>);
    }
}

#[derive(Serialize, Deserialize)]
enum NetworkMessage<T> {
    /// Sent from a client to the server to initialize communications.
    Handshake,
    /// Sent from the server to the client to respond to a handshake.
    Clients(Vec<SocketAddr>),
    /// Any message sent to any client.
    Message(T),
}

#[derive(Resource)]
struct Networking {
    receiver: Mutex<mpsc::Receiver<(Endpoint, Vec<u8>)>>,
    handler: NodeHandler<()>,
    clients: Vec<SocketAddr>,
    authority: bool,
}

fn update<'de, T: DeserializeOwned + Serialize>(mut networking: ResMut<Networking>) {
    let lock = networking.receiver.lock().unwrap();
    let mut data = Vec::new();
    while let Ok(message) = lock.try_recv() {
        data.push(message);
    }
    drop(lock);

    for (endpoint, data) in data {
        match bincode::deserialize::<NetworkMessage<T>>(&data) {
            Ok(NetworkMessage::Handshake) => {
                if networking.clients.contains(&endpoint.addr()) {
                    warn!("Received handshake from client which is already registered.");
                    continue;
                }

                if !networking.authority {
                    warn!("Received a handshake but I am not the authority.");
                    continue;
                }

                info!("Received handshake from {:?}", endpoint.addr());

                networking
                    .handler
                    .network()
                    .connect(Transport::Udp, endpoint.addr())
                    .unwrap();

                info!("Sending client list to {}", endpoint.addr());
                networking.handler.network().send(
                    endpoint,
                    &bincode::serialize(&NetworkMessage::<T>::Clients(
                        networking
                            .clients
                            .clone()
                            .into_iter()
                            .filter(|c| *c != endpoint.addr())
                            .collect(),
                    ))
                    .unwrap(),
                );

                networking.clients.push(endpoint.addr());
            }
            Ok(NetworkMessage::Clients(clients)) => {
                if networking.authority {
                    warn!(
                        "Received client list from non-authority ({})",
                        endpoint.addr()
                    );
                } else {
                    info!("Received client list: {:?}", clients);
                }

                for client in clients {
                    if networking.clients.contains(&client) {
                        continue;
                    }

                    networking
                        .handler
                        .network()
                        .connect(Transport::Udp, client)
                        .unwrap();
                }
            }
            Ok(NetworkMessage::Message(message)) => {}
            Err(error) => {
                error!(
                    "Failed to parse incoming data from {}: {}",
                    endpoint.addr(),
                    error
                );
            }
        }
    }
}
