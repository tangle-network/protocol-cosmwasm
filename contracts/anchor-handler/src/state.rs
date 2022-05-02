use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct UpdateRecord {
    pub token_addr: Addr,
    pub  src_chain_id: u64,
    pub resource_id: [u8; 32],
    pub merkle_root: [u8; 32],
    pub leaf_id: u64,
}


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub bridge_addr: Addr,
}

pub const STATE: Item<State> = Item::new("state");

/* -----  Handlers common ----- */
// resourceID => token contract address
pub const RESOURCEID2CONTRACTADDRESS: Map<&[u8], Addr> = Map::new("resourceIDToContractAddress");

// Execution contract address => resourceID
pub const CONTRACTADDRESS2RESOURCEID: Map<Addr, [u8; 32]> = Map::new("contractAddressToResourceID");

// Execution contract address => is whitelisted
pub const CONTRACTWHITELIST: Map<Addr, bool> = Map::new("contract_whitelist");
/* --------------------------- */

// sourceChainID => height => Update Record
pub const UPDATE_RECORDS: Map<(u64, u64), UpdateRecord> = Map::new("update_records");

// source chain ID => number of updates
pub const COUNTS: Map<u64, u64> = Map::new("counts");

