use crate::contract::*;
use cosmwasm_std::testing::{
    mock_dependencies, mock_env, mock_info, MockApi, MockQuerier, MockStorage,
};
use cosmwasm_std::{attr, from_binary, to_binary, OwnedDeps};

use protocol_cosmwasm::error::ContractError;
use protocol_cosmwasm::structs::BridgeAddrResponse;
use protocol_cosmwasm::treasury::ExecuteMsg as TreasuryExecuteMsg;
use protocol_cosmwasm::treasury_handler::{ExecuteMsg, InstantiateMsg, QueryMsg};

const BRIDGE_ADDR: &str = "bridge-contract";
const RESOURCE_ID: [u8; 32] = [1u8; 32];
const CONTRACT_ADDRESS: &str = "terra1jrj2vh6cstqwk3pg8nkmdf0r9z0n3q3f3jk5xn";

fn instantiate_treasury_handler() -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
    let mut deps = mock_dependencies();

    // Instantiate the "treasury-handler".
    let msg = InstantiateMsg {
        bridge_addr: BRIDGE_ADDR.to_string(),
        initial_resource_ids: vec![],
        initial_contract_addresses: vec![],
    };
    let info = mock_info("creator", &[]);
    let _ = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    deps
}

fn proposal_to_exec_data(resource_id: [u8; 32], proposal: TreasuryExecuteMsg) -> Vec<u8> {
    let base64_encoded_proposal = to_binary(&proposal).unwrap().0;

    let mut execution_data: Vec<u8> = vec![];
    execution_data.extend_from_slice(&resource_id);
    execution_data.extend_from_slice(&base64_encoded_proposal);
    execution_data
}

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies();

    let msg = InstantiateMsg {
        bridge_addr: BRIDGE_ADDR.to_string(),
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
    assert_eq!(bridge_addr_resp.bridge_addr, BRIDGE_ADDR.to_string());
}

#[test]
fn test_hander_set_resource() {
    // Instantiate the "treasury_handler"
    let mut deps = instantiate_treasury_handler();

    // Try to "set resource" from non-bridge address
    let set_resource_msg = ExecuteMsg::SetResource {
        resource_id: RESOURCE_ID,
        contract_addr: CONTRACT_ADDRESS.to_string(),
    };
    let info = mock_info("non-bridge", &[]);
    let err = execute(deps.as_mut(), mock_env(), info, set_resource_msg.clone()).unwrap_err();
    assert_eq!(err.to_string(), ContractError::Unauthorized {}.to_string());

    // "Set resource" by bridge address
    let info = mock_info(BRIDGE_ADDR, &[]);
    let res = execute(deps.as_mut(), mock_env(), info, set_resource_msg).unwrap();
    assert_eq!(res.attributes, vec![attr("method", "set_resource")]);
}

#[test]
fn test_handler_migrate_bridge() {
    // Instantiate the "treasury_handler"
    let mut deps = instantiate_treasury_handler();

    let new_bridge = "new-bridge";

    // Try to "migrate bridge" from non-bridge address
    let migrate_bridge_msg = ExecuteMsg::MigrateBridge {
        new_bridge: new_bridge.to_string(),
    };
    let info = mock_info("non-bridge", &[]);
    let err = execute(deps.as_mut(), mock_env(), info, migrate_bridge_msg.clone()).unwrap_err();
    assert_eq!(err.to_string(), ContractError::Unauthorized {}.to_string());

    // "Migrate bridge" by bridge address
    let info = mock_info(BRIDGE_ADDR, &[]);
    let res = execute(deps.as_mut(), mock_env(), info, migrate_bridge_msg).unwrap();
    assert_eq!(res.attributes, vec![attr("method", "migrate_bridge")]);

    // it worked, let's query the state("bridge_addr")
    let res = query(deps.as_ref(), mock_env(), QueryMsg::GetBridgeAddress {}).unwrap();
    let bridge_addr_resp: BridgeAddrResponse = from_binary(&res).unwrap();
    assert_eq!(bridge_addr_resp.bridge_addr, new_bridge.to_string());
}

#[test]
fn test_handler_execute_proposal() {
    // Instantiate the "treasury_handler"
    let mut deps = instantiate_treasury_handler();

    // Set the "resource_id"
    let info = mock_info(BRIDGE_ADDR, &[]);
    let set_resource_msg = ExecuteMsg::SetResource {
        resource_id: RESOURCE_ID,
        contract_addr: "treasury-contract".to_string(),
    };
    let _res = execute(deps.as_mut(), mock_env(), info, set_resource_msg).unwrap();

    // Try to set a new handler for tokenwrapper contract
    let info = mock_info(BRIDGE_ADDR, &[]);

    let set_handler_proposal = TreasuryExecuteMsg::SetHandler {
        handler: "new-handler".to_string(),
        nonce: 2u32,
    };
    let exec_data = proposal_to_exec_data(RESOURCE_ID, set_handler_proposal);
    let exec_proposal_msg = ExecuteMsg::ExecuteProposal {
        resource_id: RESOURCE_ID,
        data: exec_data,
    };

    let res = execute(deps.as_mut(), mock_env(), info, exec_proposal_msg).unwrap();
    assert_eq!(res.messages.len(), 1);
}