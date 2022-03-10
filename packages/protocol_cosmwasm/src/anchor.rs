use cosmwasm_std::{Uint128, Uint256};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use cw20::Cw20ReceiveMsg;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub max_edges: u32,
    pub chain_id: u64,
    pub levels: u32,
    pub deposit_size: Uint128,
    pub cw20_address: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Deposit(DepositMsg),
    Withdraw(WithdrawMsg),
    Receive(Cw20ReceiveMsg),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct DepositMsg {
    pub from: Option<String>,
    pub commitment: Option<[u8; 32]>,
    pub value: Uint256,
}


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw20HookMsg {
    /// Depcosit Cw20 tokens
    DepositCw20 {
        commitment: Option<[u8; 32]>,
    },
}


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct WithdrawMsg {
    pub proof_bytes: Vec<u8>,
    pub roots: Vec<[u8; 32]>,
    pub nullifier_hash: [u8; 32],
    pub recipient: String,
    pub relayer: String,
    pub fee: Uint256,
    pub refund: Uint256,
    pub commitment: [u8; 32],
    pub cw20_address: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetCw20Address {},
}
