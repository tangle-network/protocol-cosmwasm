use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};

/// Config
/// Governance - related params
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, JsonSchema)]
pub struct Config {
    pub governor: Addr,
    pub native_token_denom: String,
    pub fee_recipient: Addr,
    pub fee_percentage: u8,
    pub is_native_allowed: bool,
    pub wrapping_limit: Uint128,
    pub proposal_nonce: u64,
}

pub const CONFIG: Item<Config> = Item::new("config");

pub const TOKENS: Map<Addr, bool> = Map::new("tokens");
pub const HISTORICAL_TOKENS: Map<Addr, bool> = Map::new("historical_tokens");
