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
    /* ---  Handler common utils --- */
    SetResource {
        resource_id: [u8; 32],
        contract_addr: String,
    },

    MigrateBridge {
        new_bridge: String,
    },
    /* ----------------------------- */
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetBridgeAddress {},
    GetContractAddress { resource_id: [u8; 32] },
    GetResourceId { contract_addr: String },
    IsContractWhitelisted { contract_addr: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct BridgeAddrResponse {
    pub bridge_addr: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ContractAddrResponse {
    pub contract_addr: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ResourceIdResponse {
    pub resource_id: [u8; 32],
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct WhitelistCheckResponse {
    pub contract_addr: String,
    pub is_whitelisted: bool,
}
