use crate::contract::*;
use cosmwasm_std::testing::{
    mock_dependencies, mock_env, mock_info, MockApi, MockQuerier, MockStorage,
};
use cosmwasm_std::{attr, from_binary, OwnedDeps};

use protocol_cosmwasm::anchor_handler::{BridgeAddrResponse, ExecuteMsg, InstantiateMsg, QueryMsg};
use protocol_cosmwasm::error::ContractError;

const BRIDGE: &str = "bridge-contract";
const RESOURCE_ID: [u8; 32] = [1u8; 32];
const CONTRACT_ADDRESS: &str = "contract-address";

fn instantiate_anchor_handler() -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
    let mut deps = mock_dependencies(&[]);

    // Instantiate the "anchor-handler".
    let msg = InstantiateMsg {
        bridge_addr: BRIDGE.to_string(),
        initial_resource_ids: vec![],
        initial_contract_addresses: vec![],
    };
    let info = mock_info("creator", &[]);
    let _ = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    deps
}

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {
        bridge_addr: BRIDGE.to_string(),
        initial_resource_ids: vec![],
        initial_contract_addresses: vec![],
    };
    let info = mock_info("creator", &[]);

    // we can just call .unwrap() to assert this was a success
    let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());
    assert_eq!(res.attributes, vec![attr("method", "instantiate")]);

    // it worked, let's query the state("bridge_addr")
    let res = query(deps.as_ref(), mock_env(), QueryMsg::GetBridgeAddress {}).unwrap();
    let bridge_addr_resp: BridgeAddrResponse = from_binary(&res).unwrap();
    assert_eq!(bridge_addr_resp.bridge_addr, BRIDGE.to_string());
}

#[test]
fn test_hander_set_resource() {
    // Instantiate the "anchor_handler"
    let mut deps = instantiate_anchor_handler();

    // Try to "set resource" from non-bridge address
    let set_resource_msg = ExecuteMsg::SetResource {
        resource_id: RESOURCE_ID,
        contract_addr: CONTRACT_ADDRESS.to_string(),
    };
    let info = mock_info("non-bridge", &[]);
    let err = execute(deps.as_mut(), mock_env(), info, set_resource_msg.clone()).unwrap_err();
    assert_eq!(err.to_string(), ContractError::Unauthorized {}.to_string());

    // "Set resource" by bridge address
    let info = mock_info(BRIDGE, &[]);
    let res = execute(deps.as_mut(), mock_env(), info, set_resource_msg).unwrap();
    assert_eq!(res.attributes, vec![attr("method", "set_resource")]);
}

#[test]
fn test_handler_migrate_bridge() {
    // Instantiate the "anchor_handler"
    let mut deps = instantiate_anchor_handler();

    let new_bridge = "new-bridge";

    // Try to "migrate bridge" from non-bridge address
    let migrate_bridge_msg = ExecuteMsg::MigrateBridge {
        new_bridge: new_bridge.to_string(),
    };
    let info = mock_info("non-bridge", &[]);
    let err = execute(deps.as_mut(), mock_env(), info, migrate_bridge_msg.clone()).unwrap_err();
    assert_eq!(err.to_string(), ContractError::Unauthorized {}.to_string());

    // "Migrate bridge" by bridge address
    let info = mock_info(BRIDGE, &[]);
    let res = execute(deps.as_mut(), mock_env(), info, migrate_bridge_msg).unwrap();
    assert_eq!(res.attributes, vec![attr("method", "migrate_bridge")]);

    // it worked, let's query the state("bridge_addr")
    let res = query(deps.as_ref(), mock_env(), QueryMsg::GetBridgeAddress {}).unwrap();
    let bridge_addr_resp: BridgeAddrResponse = from_binary(&res).unwrap();
    assert_eq!(bridge_addr_resp.bridge_addr, new_bridge.to_string());
}
