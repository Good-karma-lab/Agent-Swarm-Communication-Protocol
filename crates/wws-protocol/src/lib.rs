//! WWS Protocol - Core types and message definitions
//!
//! Implements the World Wide Swarm (WWS) message specification
//! using JSON-RPC 2.0 envelope format with Ed25519 signatures.

pub mod identity;
pub mod messages;
pub mod types;
pub mod error;
pub mod constants;
pub mod crypto;
pub mod key_rotation;

pub use identity::*;
pub use messages::*;
pub use types::*;
pub use error::*;
pub use constants::*;
pub use key_rotation::{
    RotationAnnouncement,
    EmergencyRevocation,
    GuardianDesignation,
    GuardianRecoveryVote,
    verify_guardian_threshold,
    build_rotation_announcement,
    verify_rotation_announcement,
    current_timestamp_secs,
    ROTATION_TIMESTAMP_TOLERANCE_SECS,
};
