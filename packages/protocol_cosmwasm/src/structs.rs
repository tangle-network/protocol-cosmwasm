use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// History length of merkle tree root
pub const ROOT_HISTORY_SIZE: u32 = 100;

// ChainType info
pub const COSMOS_CHAIN_TYPE: [u8; 2] = [4, 0]; // 0x0400

// History length for the "curr_neighbor_root_index".
pub const HISTORY_LENGTH: u32 = 30;

pub type ChainId = u64;
pub type Element = [u8; 32];
pub type LatestLeafIndex = u32;

// Edge: Directed connection or link between two anchors.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Default, Copy)]
pub struct Edge {
    /// chain id
    pub src_chain_id: ChainId,
    /// root of source chain anchor's native merkle tree
    pub root: Element,
    /// height of source chain anchor's native merkle tree
    pub latest_leaf_index: LatestLeafIndex,
    /// Target contract address or tree identifier
    pub target: Element,
}
