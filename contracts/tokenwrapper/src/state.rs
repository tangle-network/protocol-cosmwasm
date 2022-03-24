use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::Uint128;
use cw_storage_plus::Item;

/// Supply is dynamic and tracks the current supply of staked and ERC20 tokens.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Default)]
pub struct Supply {
    /// issued is how many wrapped tokens this contract has issued
    pub issued: Uint128,
}

pub const TOTAL_SUPPLY: Item<Supply> = Item::new("total_supply");
