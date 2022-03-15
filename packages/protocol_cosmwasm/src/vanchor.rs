use cosmwasm_std::{Uint256};
use cw20::Cw20ReceiveMsg;
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
    Receive(Cw20ReceiveMsg),
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
pub enum Cw20HookMsg {
    Transact { 
        proof_data: ProofData,
        ext_data: ExtData,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ProofData {
    pub proof: Vec<u8>,
    pub public_amount: [u8; 32],
    pub roots: Vec<[u8; 32]>,
    pub input_nullifiers: Vec<[u8; 32]>,
    pub output_commitments: Vec<[u8; 32]>,
    pub ext_data_hash: [u8; 32],
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ExtData {
    pub recipient: String,
    pub relayer: String,
    pub ext_amount: [u8; 32],
    pub fee: [u8; 32],
    pub encrypted_output1: [u8; 32],
    pub encrypted_output2: [u8; 32],
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
   // TODO
}

