use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct State {
    pub governor: Vec<u8>,
    pub proposal_nonce: u32,
}

pub const STATE: Item<State> = Item::new("state");

/// destinationChainID => number of deposits
pub const COUNTS: Map<&[u8], [u8; 32]> = Map::new("counts");

/// resourceID => handler address
pub const RESOURCEID2HANDLERADDR: Map<&[u8], String> = Map::new("resourceIDToHandlerAddress");
