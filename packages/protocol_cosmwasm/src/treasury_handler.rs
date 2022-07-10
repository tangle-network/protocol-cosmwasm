use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// @dev {initial_resource_ids} and {initial_contract_addresses} must have the same length (one resourceID for every address).
// Also, these arrays must be ordered in the way that {initial_resource_ids}[0] is the intended resourceID for {initial_contract_addresses}[0].
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    SetResource {
        resource_id: [u8; 32],
        contract_addr: String,
    },

    MigrateBridge {
        new_bridge: String,
    },

    ExecuteProposal {
        resource_id: [u8; 32],
        data: Vec<u8>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetBridgeAddress {},
    GetContractAddress {
        resource_id: [u8; 32],
    },
    GetResourceId {
        contract_addr: String,
    },
    IsContractWhitelisted {
        contract_addr: String,
    },
    GetUpdateRecord {
        update_nonce: u64,
        src_chain_id: u64,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct UpdateRecordResponse {
    pub treasury_addr: String,
    pub exec_chain_id: u64,
    pub nonce: u64,
    pub resource_id: [u8; 32],
    pub update_value: [u8; 32],
}