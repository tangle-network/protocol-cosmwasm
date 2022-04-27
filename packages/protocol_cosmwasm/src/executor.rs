use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Interface for handler contracts that support proposal executions.

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    // It is intended that proposals are executed by the Bridge contract.
    ExecuteProposal {
        resource_id: [u8; 32],
        data: Vec<u8>,
    },

    // Correlates {resourceID} with {contractAddress}.
    SetResource {
        resource_id: [u8; 32],
        contract_addr: String,
    },

    // Migrates the bridge to a new bridge address.
    MigrateBridge {
        new_bridge: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {}
