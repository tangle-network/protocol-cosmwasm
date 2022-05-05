use cw20::Cw20ReceiveMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub max_edges: u32,
    pub chain_id: u64,
    pub levels: u32,
    pub deposit_size: String,
    pub tokenwrapper_addr: String,
    pub handler: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Withdraw a deposit from the contract
    Withdraw(WithdrawMsg),

    /// Unwraps the Anchor's TokenWrapper token for the `sender`
    /// into one of its wrappable tokens.
    UnwrapIntoToken { token_addr: String, amount: String },

    /// Wraps the native token to "TokenWrapper" token
    WrapNative { amount: String },
    /// Unwraps the "TokenWrapper" token to native token
    UnwrapNative { amount: String },

    /// Wraps the native token & deposit it into the contract
    WrapAndDeposit {
        commitment: Option<[u8; 32]>,
        amount: String,
    },

    /// Withdraws the deposit & unwraps into valid token for `sender`
    WithdrawAndUnwrap(WithdrawMsg),

    /// Handles the cw20 token receive cases
    /// 1. DepositCw20
    /// 2. WrapToken
    Receive(Cw20ReceiveMsg),

    /// Add an edge to underlying tree
    AddEdge {
        src_chain_id: u64,
        root: [u8; 32],
        latest_leaf_index: u32,
        target: [u8; 32],
    },

    /// Update an edge for underlying tree
    UpdateEdge {
        src_chain_id: u64,
        root: [u8; 32],
        latest_leaf_index: u32,
        target: [u8; 32],
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw20HookMsg {
    /// Depcosit Cw20 tokens
    DepositCw20 { commitment: Option<[u8; 32]> },

    /// Wraps a cw20 token for the `sender` using
    /// the underlying Anchor's TokenWrapper contract
    WrapToken {},

    /// Wraps a cw20 token for the `sender`
    /// & deposit it into the contract.
    WrapAndDeposit {
        commitment: Option<[u8; 32]>,
        amount: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct WithdrawMsg {
    pub proof_bytes: Vec<u8>,
    pub roots: Vec<[u8; 32]>,
    pub nullifier_hash: [u8; 32],
    pub recipient: String,
    pub relayer: String,
    pub fee: String,
    pub refund: String,
    pub commitment: [u8; 32],
    pub cw20_address: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
    EdgeInfo { id: u64 },
    NeighborRootInfo { chain_id: u64, id: u32 },
    MerkleTreeInfo {},
    MerkleRootInfo { id: u32 },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub tokenwrapper_addr: String,
    pub chain_id: u64,
    pub deposit_size: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct EdgeInfoResponse {
    pub src_chain_id: u64,
    pub root: [u8; 32],
    pub latest_leaf_index: u32,
    pub target: [u8; 32],
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct NeighborRootInfoResponse {
    pub neighbor_root: [u8; 32],
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MerkleTreeInfoResponse {
    pub levels: u32,
    pub curr_root_index: u32,
    pub next_index: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MerkleRootInfoResponse {
    pub root: [u8; 32],
}
