use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, StdResult, Storage};
use cw_storage_plus::{Item, Map};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct UpdateRecord {
    pub treasury_addr: Addr,
    pub exec_chain_id: u64,
    pub nonce: u64,
    pub resource_id: [u8; 32],
    pub update_value: [u8; 32],
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct State {
    pub bridge_addr: Addr,
}

pub const STATE: Item<State> = Item::new("state");

/* -----  Handlers common ----- */
/// resourceID => token contract address
pub const RESOURCEID2CONTRACTADDRESS: Map<&[u8], Addr> = Map::new("resourceIDToContractAddress");

/// Execution contract address => resourceID
pub const CONTRACTADDRESS2RESOURCEID: Map<Addr, [u8; 32]> = Map::new("contractAddressToResourceID");

/// Execution contract address => is whitelisted
pub const CONTRACTWHITELIST: Map<Addr, bool> = Map::new("contract_whitelist");

pub fn set_resource(
    store: &mut dyn Storage,
    resource_id: [u8; 32],
    contract_addr: Addr,
) -> StdResult<()> {
    RESOURCEID2CONTRACTADDRESS.save(store, &resource_id, &contract_addr)?;
    CONTRACTADDRESS2RESOURCEID.save(store, contract_addr.clone(), &resource_id)?;
    CONTRACTWHITELIST.save(store, contract_addr, &true)
}

pub fn read_contract_addr(store: &dyn Storage, resource_id: [u8; 32]) -> StdResult<Addr> {
    RESOURCEID2CONTRACTADDRESS.load(store, &resource_id)
}

pub fn read_resource_id(store: &dyn Storage, contract_addr: Addr) -> StdResult<[u8; 32]> {
    CONTRACTADDRESS2RESOURCEID.load(store, contract_addr)
}

pub fn read_whitelist(store: &dyn Storage, contract_addr: Addr) -> StdResult<bool> {
    CONTRACTWHITELIST.load(store, contract_addr)
}
/* --------------------------- */

/* ---------- Treasury-Handler specific DS ----------  */
/// sourceChainID => nonce => Update Record
/// (src_chain_id, nonce) -> UpdateRecord
pub const UPDATE_RECORDS: Map<(String, String), UpdateRecord> = Map::new("update_records");

/// source chain ID => number of updates
pub const COUNTS: Map<u64, u64> = Map::new("counts");

pub fn read_update_record(
    store: &dyn Storage,
    src_chain_id: u64,
    nonce: u64,
) -> StdResult<UpdateRecord> {
    UPDATE_RECORDS.load(store, (src_chain_id.to_string(), nonce.to_string()))
}
/* ------------------------------------------------- */
