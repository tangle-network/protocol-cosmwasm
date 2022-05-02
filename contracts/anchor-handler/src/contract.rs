#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult,
};
use cw2::set_contract_version;

use crate::state::{set_resource, State, STATE};
use protocol_cosmwasm::anchor_handler::{BridgeAddrResponse, ExecuteMsg, InstantiateMsg, QueryMsg};
use protocol_cosmwasm::error::ContractError;
use protocol_cosmwasm::keccak::Keccak256;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cosmwasm-anchor-handler";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// ChainType info
pub const COSMOS_CHAIN_TYPE: [u8; 2] = [4, 0]; // 0x0400

pub const MOCK_CHAIN_ID: u64 = 1;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Validations
    if !info.funds.is_empty() {
        return Err(ContractError::UnnecessaryFunds {});
    }

    if msg.initial_resource_ids.len() != msg.initial_contract_addresses.len() {
        return Err(ContractError::Std(StdError::GenericErr {
            msg: "initial_resource_ids and initial_contract_addresses len mismatch".to_string(),
        }));
    }

    // Set "state"
    let bridge_addr = deps.api.addr_validate(&msg.bridge_addr)?;
    STATE.save(deps.storage, &State { bridge_addr })?;

    // Save the initial mapping of `resource_id => contract_addr`
    let n = msg.initial_resource_ids.len();
    for i in 0..n {
        let resource_id = msg.initial_resource_ids[i];
        let contract_addr = deps.api.addr_validate(&msg.initial_contract_addresses[i])?;
        set_resource(deps.storage, resource_id, contract_addr)?;
    }

    Ok(Response::new().add_attributes(vec![attr("method", "instantiate")]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        // TODO
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetBridgeAddress {} => to_binary(&get_bridge_addr(deps)?),
    }
}

// Query the "bridge_addr" from "State".
fn get_bridge_addr(deps: Deps) -> StdResult<BridgeAddrResponse> {
    let bridge_addr = STATE.load(deps.storage)?.bridge_addr.to_string();
    Ok(BridgeAddrResponse { bridge_addr })
}

// Verifying signature of governor over some datahash
fn signed_by_governor(
    deps: DepsMut,
    data: &[u8],
    sig: &[u8],
    governor: &str,
) -> Result<bool, ContractError> {
    let hashed_data = Keccak256::hash(data).map_err(|_| ContractError::HashError)?;
    let verify_result = deps
        .api
        .secp256k1_verify(&hashed_data, sig, governor.as_bytes());

    verify_result.map_err(|e| ContractError::Std(StdError::VerificationErr { source: e }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{attr, from_binary};

    const BRIDGE: &str = "bridge-contract";

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
        assert_eq!(res.attributes, vec![attr("method", "instantiate"),]);

        // it worked, let's query the state("bridge_addr")
        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetBridgeAddress {}).unwrap();
        let bridge_addr_resp: BridgeAddrResponse = from_binary(&res).unwrap();
        assert_eq!(bridge_addr_resp.bridge_addr, BRIDGE.to_string());
    }
}
