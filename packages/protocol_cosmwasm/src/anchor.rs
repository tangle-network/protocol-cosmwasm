use cw20::Cw20ReceiveMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub max_edges: u32,
    pub chain_id: u64,
    pub levels: u32,
    pub deposit_size: String,
    pub cw20_address: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Withdraw(WithdrawMsg),
    Receive(Cw20ReceiveMsg),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw20HookMsg {
    /// Depcosit Cw20 tokens
    DepositCw20 { commitment: Option<[u8; 32]> },
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
    pub cw20_address: String,
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
    pub cw20_address: String,
    pub chain_id: u64,
    pub deposit_size: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct EdgeInfoResponse {
    pub chain_id: u64,
    pub root: [u8; 32],
    pub latest_leaf_index: u32,
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
