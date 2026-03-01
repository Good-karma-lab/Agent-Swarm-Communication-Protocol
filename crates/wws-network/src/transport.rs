//! Transport configuration using TCP + QUIC + Noise + Yamux + Circuit Relay + DCUtR.
//!
//! Builds a libp2p Swarm using the SwarmBuilder API with:
//! - TCP transport for reliable connections
//! - QUIC transport for low-latency UDP connections
//! - Noise protocol for authenticated encryption
//! - Yamux for stream multiplexing
//! - Circuit relay client for NAT traversal via relay nodes
//! - DCUtR (hole-punching) for direct peer connections through NAT
//! - Optional idle connection timeout

use std::time::Duration;

use libp2p::Swarm;

use crate::behaviour::{BehaviourConfig, SwarmBehaviour};
use crate::NetworkError;

/// Configuration for the transport layer.
#[derive(Debug, Clone)]
pub struct TransportConfig {
    /// How long an idle connection stays open before being closed.
    pub idle_connection_timeout: Duration,
    /// Behaviour configuration.
    pub behaviour_config: BehaviourConfig,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            idle_connection_timeout: Duration::from_secs(60),
            behaviour_config: BehaviourConfig::default(),
        }
    }
}

/// Build a fully configured libp2p Swarm with TCP + QUIC transport, circuit relay client,
/// and DCUtR hole-punching alongside the composite WWS behaviour.
///
/// The swarm is created with a fresh identity. Returns the swarm ready
/// for listening and dialing.
pub fn build_swarm(config: TransportConfig) -> Result<Swarm<SwarmBehaviour>, NetworkError> {
    let keypair = libp2p::identity::Keypair::generate_ed25519();
    build_swarm_inner(keypair, config)
}

/// Build a swarm with an existing identity keypair.
///
/// Useful when restoring a node's identity from persistent storage.
pub fn build_swarm_with_keypair(
    keypair: libp2p::identity::Keypair,
    config: TransportConfig,
) -> Result<Swarm<SwarmBehaviour>, NetworkError> {
    build_swarm_inner(keypair, config)
}

/// Internal helper: build the swarm from a keypair + config.
///
/// The builder chain is:
/// 1. `.with_tcp()` — reliable byte-stream transport
/// 2. `.with_quic()` — low-latency UDP transport (alongside TCP)
/// 3. `.with_relay_client()` — circuit relay transport + behaviour
/// 4. `.with_behaviour(|key, relay_client| ...)` — composite WWS behaviour
///
/// The `relay_client` is produced by step 3 and threaded into the behaviour
/// constructor, where it is stored as a field in `SwarmBehaviour`.
fn build_swarm_inner(
    keypair: libp2p::identity::Keypair,
    config: TransportConfig,
) -> Result<Swarm<SwarmBehaviour>, NetworkError> {
    let behaviour_config = config.behaviour_config.clone();

    let swarm = libp2p::SwarmBuilder::with_existing_identity(keypair)
        .with_tokio()
        // Step 1: TCP transport with Noise + Yamux.
        .with_tcp(
            libp2p::tcp::Config::default(),
            libp2p::noise::Config::new,
            libp2p::yamux::Config::default,
        )
        .map_err(|e| NetworkError::Transport(format!("TCP transport error: {e}")))?
        // Step 2: QUIC transport (UDP), added alongside TCP.
        .with_quic()
        // Step 3: Circuit relay client transport + behaviour.
        // The relay client behaviour is returned from the builder and passed
        // into the with_behaviour closure as the second argument.
        .with_relay_client(
            libp2p::noise::Config::new,
            libp2p::yamux::Config::default,
        )
        .map_err(|e| NetworkError::Transport(format!("Relay client transport error: {e}")))?
        // Step 4: Composite WWS behaviour.
        .with_behaviour(|key, relay_client| {
            SwarmBehaviour::new(key, &behaviour_config, relay_client)
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
        })
        .map_err(|e| NetworkError::Behaviour(format!("Behaviour init error: {e}")))?
        .with_swarm_config(|c| {
            c.with_idle_connection_timeout(config.idle_connection_timeout)
        })
        .build();

    Ok(swarm)
}
