use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub initial_governor: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    // Sets a new resource for handler contracts that use the IExecutor interface,
    // and maps the {handlerAddress} to {newResourceID} in {_resourceIDToHandlerAddress}.
    AdminSetResWithSig(SetResWithSigMsg),

    // Executes a proposal signed by the governor.
    ExecProposalWithSig(ExecProposalWithSigMsg),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct SetResWithSigMsg {
    pub resource_id: [u8; 32],
    pub function_sig: [u8; 4],
    pub nonce: u32,
    pub new_resource_id: [u8; 32],
    pub handler_addr: String,
    pub execution_context_addr: String,
    pub sig: Vec<u8>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ExecProposalWithSigMsg {
    pub data: Vec<u8>,
    pub sig: Vec<u8>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    // Get the state
    GetState {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct StateResponse {
    pub governor: String,
    pub proposal_nonce: u32,
}
