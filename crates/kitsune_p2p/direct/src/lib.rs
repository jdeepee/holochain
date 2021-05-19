//! Kitsune P2p Direct Application Framework
#![deny(warnings)]
#![deny(missing_docs)]
#![deny(unsafe_code)]

use kitsune_p2p_types::tx2::tx2_adapter::Uniq;
pub use kitsune_p2p_types::{KitsuneError, KitsuneResult};

use sodoken::Buffer;

use std::sync::Arc;

pub mod types;

mod persist_mem;
pub use persist_mem::*;

mod v1;
pub use v1::*;

/// kdirect reexported dependencies
pub mod dependencies {
    pub use futures;
    pub use kitsune_p2p;
    pub use kitsune_p2p_types;
    pub use kitsune_p2p_types::dependencies::{ghost_actor::dependencies::tracing, observability};
    pub use serde_json;
}

/// kdirect prelude
pub mod prelude {
    pub use crate::persist_mem::*;
    pub use crate::types::direct::{KitsuneDirect, KitsuneDirectEvt};
    pub use crate::types::kdentry::{KdEntry, KdEntryData};
    pub use crate::types::kdhash::KdHash;
    pub use crate::types::persist::KdPersist;
    pub use crate::v1::*;
    pub use crate::{KitsuneError, KitsuneResult};
    pub use kitsune_p2p::dht_arc::DhtArc;
}