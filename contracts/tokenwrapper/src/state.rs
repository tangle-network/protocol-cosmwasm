use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Decimal, Uint128};
use cw_storage_plus::{Item, Map};

/// Supply is dynamic and tracks the current supply of staked and ERC20 tokens.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Default)]
pub struct Supply {
    /// issued is how many wrapped tokens this contract has issued
    pub issued: Uint128,
}

/// Config
/// Governance - related params
#[derive(Serialize, Deserialize, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub governor: Addr,
    pub native_token_denom: String,
    pub fee_recipient: Addr,
    pub fee_percentage: Decimal,
    pub is_native_allowed: bool,
    pub wrapping_limit: Uint128,
    pub proposal_nonce: u64,
}

/// Normal Cw20 - related
pub const TOTAL_SUPPLY: Item<Supply> = Item::new("total_supply");

/// Governance - related
pub const CONFIG: Item<Config> = Item::new("config");

pub const TOKENS: Map<Addr, bool> = Map::new("tokens");
pub const HISTORICAL_TOKENS: Map<Addr, bool> = Map::new("historical_tokens");
