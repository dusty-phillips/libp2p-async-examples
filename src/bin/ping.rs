use async_std::task;
use libp2p::{
    build_development_transport, identity,
    ping::{Ping, PingConfig},
    swarm::SwarmEvent,
    PeerId, Swarm,
};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let id_keys = identity::Keypair::generate_ed25519();
    let peer_id = PeerId::from(id_keys.public());
    println!("Local peer id: {:?}", peer_id);

    let transport = build_development_transport(id_keys)?;
    let behaviour = Ping::new(PingConfig::new().with_keep_alive(true));

    let mut swarm = Swarm::new(transport, behaviour, peer_id);
    Swarm::listen_on(&mut swarm, "/ip4/0.0.0.0/tcp/0".parse()?)?;

    if let Some(addr) = std::env::args().nth(1) {
        let remote = addr.parse()?;
        Swarm::dial_addr(&mut swarm, remote)?;
        println!("Dialed {}", addr)
    }

    task::block_on(async {
        loop {
            let event = swarm.next_event().await;
            match event {
                SwarmEvent::NewListenAddr(address) => {
                    println!("Listening on {}", address);
                }
                SwarmEvent::Behaviour(event) => {
                    println!("{:?}", event);
                }
                _ => {}
            }
        }
    });

    Ok(())
}
