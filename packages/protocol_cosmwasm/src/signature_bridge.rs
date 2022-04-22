use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub inital_governer: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    // Sets a new resource for handler contracts that use the IExecutor interface,
    // and maps the {handlerAddress} to {newResourceID} in {_resourceIDToHandlerAddress}.
    AdminSetResWithSig,

    // Executes a proposal signed by the governor.
    ExecProposalWithSig,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    // Get the state
    GetState {},
}
