use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::Addr;
use cw_storage_plus::Item;

pub const CONFIG: Item<Config> = Item::new("config");

/// Config
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, JsonSchema)]
pub struct Config {
    pub treasury_handler: Addr,
    pub proposal_nonce: u32,
}