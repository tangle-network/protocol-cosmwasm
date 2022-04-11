use cosmwasm_std::Uint128;
use cw20::Cw20ReceiveMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub chain_id: u64,
    pub levels: u32,
    pub max_edges: u32,
    pub cw20_address: String,
    pub max_deposit_amt: Uint128,
    pub min_withdraw_amt: Uint128,
    pub max_ext_amt: Uint128,
    pub max_fee: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    UpdateConfig(UpdateConfigMsg),
    Receive(Cw20ReceiveMsg),
    TransactWithdraw {
        proof_data: ProofData,
        ext_data: ExtData,
    },
    AddEdge {
        src_chain_id: u64,
        root: [u8; 32],
        latest_leaf_index: u32,
    },
    UpdateEdge {
        src_chain_id: u64,
        root: [u8; 32],
        latest_leaf_index: u32,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct UpdateConfigMsg {
    pub max_deposit_amt: Option<Uint128>,
    pub min_withdraw_amt: Option<Uint128>,
    pub max_ext_amt: Option<Uint128>,
    pub max_fee: Option<Uint128>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw20HookMsg {
    TransactDeposit {
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

impl ProofData {
    pub fn new(
        proof: Vec<u8>,
        public_amount: [u8; 32],
        roots: Vec<[u8; 32]>,
        input_nullifiers: Vec<[u8; 32]>,
        output_commitments: Vec<[u8; 32]>,
        ext_data_hash: [u8; 32],
    ) -> Self {
        Self {
            proof,
            public_amount,
            roots,
            input_nullifiers,
            output_commitments,
            ext_data_hash,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ExtData {
    pub recipient: String,
    pub relayer: String,
    pub ext_amount: String,
    pub fee: String,
    pub encrypted_output1: [u8; 32],
    pub encrypted_output2: [u8; 32],
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    // TODO
}
