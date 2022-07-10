use cosmwasm_std::Uint128;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const WRAP_FEE_CALC_DENOMINATOR: u8 = 100_u8;
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct InstantiateMsg {
    /// Address of "treasury_handler"
    pub treasury_handler: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Send the (native or cw20) tokens to destination address
    RescueTokens {
        token_info: TokenInfo,
        to: String,
        amount_to_rescue: Uint128,
        nonce: u32,
    },

    /// Sets a new handler for contract
    SetHandler { handler: String, nonce: u32 },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Get the config
    GetConfig {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct ConfigResponse {
    pub treasury_handler: String,
    pub proposal_nonce: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TokenInfo {
    Native(String),
    Cw20(String),
}