use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// @dev {initial_resource_ids} and {initial_contract_addresses} must have the same length (one resourceID for every address).
// Also, these arrays must be ordered in the way that {initial_resource_ids}[0] is the intended resourceID for {initial_contract_addresses}[0].
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    // Contract address of previously deployed Bridge.
    pub bridge_addr: String,
    // Resource IDs are used to identify a specific contract address.
    // These are the Resource IDs this contract will initially support.
    pub initial_resource_ids: Vec<[u8; 32]>,
    // These are the addresses the {initial_resource_ids} will point to,
    // and are the contracts that will be called to perform various deposit calls.
    pub initial_contract_addresses: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    // TODO
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetBridgeAddress {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct BridgeAddrResponse {
    pub bridge_addr: String,
}
