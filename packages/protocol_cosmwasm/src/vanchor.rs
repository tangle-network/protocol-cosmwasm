use cosmwasm_std::{Uint256};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub levels: u32,
    pub max_edges: u32,
    pub cw20_address: String,
    pub max_deposit_amt: Uint256,
    pub min_withdraw_amt: Uint256,
    pub max_ext_amt: Uint256,
    pub max_fee: Uint256,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    UpdateConfig(UpdateConfigMsg),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct UpdateConfigMsg {
    pub max_deposit_amt: Option<Uint256>,
    pub min_withdraw_amt: Option<Uint256>,
    pub max_ext_amt: Option<Uint256>,
    pub max_fee: Option<Uint256>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
   // TODO
}

