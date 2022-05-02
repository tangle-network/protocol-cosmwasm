#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult,
};
use cw2::set_contract_version;

use crate::state::{
    read_contract_addr, read_resource_id, read_whitelist, set_resource, State, STATE,
};
use protocol_cosmwasm::anchor_handler::{
    BridgeAddrResponse, ContractAddrResponse, ExecuteMsg, InstantiateMsg, QueryMsg,
    ResourceIdResponse, WhitelistCheckResponse,
};
use protocol_cosmwasm::error::ContractError;

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
        /* ---  Handler common utils --- */
        ExecuteMsg::SetResource {
            resource_id,
            contract_addr,
        } => exec_set_resource(deps, info, resource_id, contract_addr),
        ExecuteMsg::MigrateBridge { new_bridge } => migrate_bridge(deps, info, new_bridge),
        /* ---------------------------- */
    }
}

fn exec_set_resource(
    deps: DepsMut,
    info: MessageInfo,
    resource_id: [u8; 32],
    contract_addr: String,
) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;

    // Validations
    if info.sender != state.bridge_addr {
        return Err(ContractError::Unauthorized {});
    }

    // Save/update the mapping `resource_id => contract_addr`
    let contract_addr = deps.api.addr_validate(&contract_addr)?;
    set_resource(deps.storage, resource_id, contract_addr)?;

    Ok(Response::new().add_attribute("method", "set_resource"))
}

fn migrate_bridge(
    deps: DepsMut,
    info: MessageInfo,
    new_bridge: String,
) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;

    // Validations
    if info.sender != state.bridge_addr {
        return Err(ContractError::Unauthorized {});
    }

    // Migrage(update) the "bridge_addr" with "new_bridge"
    let bridge_addr = deps.api.addr_validate(&new_bridge)?;
    STATE.save(deps.storage, &State { bridge_addr })?;

    Ok(Response::new().add_attribute("method", "migrate_bridge"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetBridgeAddress {} => to_binary(&get_bridge_addr(deps)?),
        QueryMsg::GetContractAddress { resource_id } => {
            to_binary(&get_contract_addr(deps, resource_id)?)
        }
        QueryMsg::GetResourceId { contract_addr } => {
            to_binary(&get_resource_id(deps, contract_addr)?)
        }
        QueryMsg::IsContractWhitelisted { contract_addr } => {
            to_binary(&is_whitelisted(deps, contract_addr)?)
        }
    }
}

// Query the "bridge_addr" from "State".
fn get_bridge_addr(deps: Deps) -> StdResult<BridgeAddrResponse> {
    let bridge_addr = STATE.load(deps.storage)?.bridge_addr.to_string();
    Ok(BridgeAddrResponse { bridge_addr })
}

// Query the "contract_addr" by "resource_id".
fn get_contract_addr(deps: Deps, resource_id: [u8; 32]) -> StdResult<ContractAddrResponse> {
    let contract_addr = read_contract_addr(deps.storage, resource_id)?.to_string();
    Ok(ContractAddrResponse { contract_addr })
}

// Query the "resource_id" by "contract_addr"
fn get_resource_id(deps: Deps, contract_addr: String) -> StdResult<ResourceIdResponse> {
    let contract_addr = deps.api.addr_validate(&contract_addr)?;
    let resource_id = read_resource_id(deps.storage, contract_addr)?;
    Ok(ResourceIdResponse { resource_id })
}

// Query if the given "contract_addr" is whitelisted
fn is_whitelisted(deps: Deps, contract_addr: String) -> StdResult<WhitelistCheckResponse> {
    let contract = deps.api.addr_validate(&contract_addr)?;
    let is_whitelisted = read_whitelist(deps.storage, contract)?;
    Ok(WhitelistCheckResponse {
        contract_addr,
        is_whitelisted,
    })
}