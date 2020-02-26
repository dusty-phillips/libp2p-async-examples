use async_std::{io, io::prelude::BufReadExt, task};
use futures::{future::FutureExt, select, stream::StreamExt};
use libp2p::{
    build_development_transport,
    floodsub::{Floodsub, FloodsubEvent, Topic},
    identity,
    mdns::{Mdns, MdnsEvent},
    swarm::{NetworkBehaviourEventProcess, SwarmEvent},
    NetworkBehaviour, PeerId, Swarm,
};
use std::error::Error;

#[derive(NetworkBehaviour)]
struct ChatBehaviour {
    floodsub: Floodsub,
    mdns: Mdns,
}

impl NetworkBehaviourEventProcess<MdnsEvent> for ChatBehaviour {
    // Called when `mdns` produces an event.
    fn inject_event(&mut self, event: MdnsEvent) {
        match event {
            MdnsEvent::Discovered(list) => {
                for (peer, _) in list {
                    println!("Discovered {:?}", peer);
                    self.floodsub.add_node_to_partial_view(peer);
                }
            }
            MdnsEvent::Expired(list) => {
                for (peer, _) in list {
                    if !self.mdns.has_node(&peer) {
                        println!("Expiring {:?}", peer);
                        self.floodsub.remove_node_from_partial_view(&peer);
                    }
                }
            }
        }
    }
}

impl NetworkBehaviourEventProcess<FloodsubEvent> for ChatBehaviour {
    // Called when `floodsub` produces an event.
    fn inject_event(&mut self, message: FloodsubEvent) {
        if let FloodsubEvent::Message(message) = message {
            println!(
                "Received: '{:?}' from {:?}",
                String::from_utf8_lossy(&message.data),
                message.source
            );
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let id_keys = identity::Keypair::generate_ed25519();
    let peer_id = PeerId::from(id_keys.public());
    println!("Local peer id: {:?}", peer_id);

    let transport = build_development_transport(id_keys)?;

    let floodsub_topic = Topic::new("chat");
    let mut behaviour = ChatBehaviour {
        mdns: Mdns::new()?,
        floodsub: Floodsub::new(peer_id.clone()),
    };
    behaviour.floodsub.subscribe(floodsub_topic.clone());

    let mut swarm = Swarm::new(transport, behaviour, peer_id);
    Swarm::listen_on(&mut swarm, "/ip4/0.0.0.0/tcp/0".parse()?)?;

    if let Some(addr) = std::env::args().nth(1) {
        let remote = addr.parse()?;
        Swarm::dial_addr(&mut swarm, remote)?;
        println!("Dialed {}", addr)
    }

    let mut standard_in = io::BufReader::new(io::stdin()).lines();

    task::block_on(async {
        loop {
            select! {
                event = swarm.next_event().fuse() => {
                    match event {
                        SwarmEvent::NewListenAddr(address) => {
                            println!("Listening on {}", address);
                        }
                        _ => {}
                    }
                }
                line = standard_in.next().fuse() => {
                    swarm.floodsub.publish(floodsub_topic.clone(), line.unwrap().unwrap().as_bytes());
                }
            }
        }
    });

    Ok(())
}
