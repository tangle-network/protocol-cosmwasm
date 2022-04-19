use cosmwasm_std::Uint128;
use cw20::Cw20ReceiveMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub chain_id: u64,
    pub levels: u32,
    pub max_edges: u32,
    pub tokenwrapper_addr: String,
    pub max_deposit_amt: Uint128,
    pub min_withdraw_amt: Uint128,
    pub max_ext_amt: Uint128,
    pub max_fee: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Update the config
    UpdateConfig(UpdateConfigMsg),

    /// Handles the cw20 token receive cases
    /// 1. Executes a deposit or combination join/split transaction
    /// 2. WrapToken
    Receive(Cw20ReceiveMsg),

    /// Executes a withdrawal or combination join/split transaction
    TransactWithdraw {
        proof_data: ProofData,
        ext_data: ExtData,
    },

    /// Wraps the native token to "TokenWrapper" token
    WrapNative { amount: String },

    /// Unwraps the "TokenWrapper" token to native token
    UnwrapNative { amount: String },

    /// Unwraps the VAnchor's TokenWrapper token for the `sender`
    /// into one of its wrappable tokens.
    UnwrapIntoToken { token_addr: String, amount: String },

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
    /// Executes a deposit or combination join/split transaction
    TransactDeposit {
        proof_data: ProofData,
        ext_data: ExtData,
    },

    /// Wraps cw20 token for the `sender` using
    /// the underlying VAnchor's TokenWrapper contract
    WrapToken {},
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
