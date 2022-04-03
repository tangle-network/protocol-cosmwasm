use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Decimal, Uint128};
use cw_storage_plus::Item;

/// Supply is dynamic and tracks the current supply of staked and ERC20 tokens.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Default)]
pub struct Supply {
    /// issued is how many wrapped tokens this contract has issued
    pub issued: Uint128,
}

/// Config tracks native token denom
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub governer: Addr,
    pub native_token_denom: String,
    pub fee_recipient: Addr,
    pub fee_percentage: Decimal,
}

pub const TOTAL_SUPPLY: Item<Supply> = Item::new("total_supply");
pub const CONFIG: Item<Config> = Item::new("config");
