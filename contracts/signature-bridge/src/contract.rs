#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, to_binary, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response, StdError,
    StdResult, WasmMsg,
};
use cw2::set_contract_version;

use crate::state::{State, RESOURCEID2HANDLERADDR, STATE};
use protocol_cosmwasm::error::ContractError;
use protocol_cosmwasm::executor::ExecuteMsg as ExecutorExecMsg;
use protocol_cosmwasm::signature_bridge::{
    ExecProposalWithSigMsg, ExecuteMsg, InstantiateMsg, QueryMsg, SetResourceWithSigMsg,
    StateResponse,
};
use protocol_cosmwasm::utils::{
    compute_chain_id, compute_chain_id_type, element_encoder, get_chain_id_type,
};
// Essentially, this is from "tiny_keccak" crate.
use arkworks_setups::common::keccak_256;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cosmwasm-signature-bridge";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// ChainType info
pub const COSMOS_CHAIN_TYPE: [u8; 2] = [4, 0]; // 0x0400

const COMPRESSED_PUBKEY_LEN: usize = 33;
const UNCOMPRESSED_PUBKEY_LEN: usize = 65;

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

    if msg.initial_governor.len() != COMPRESSED_PUBKEY_LEN
        && msg.initial_governor.len() != UNCOMPRESSED_PUBKEY_LEN
    {
        return Err(ContractError::Std(StdError::generic_err(
            "Pubkey length does not match.",
        )));
    }

    // Set "state"
    STATE.save(
        deps.storage,
        &State {
            governor: msg.initial_governor,
            proposal_nonce: 0,
        },
    )?;

    Ok(Response::new().add_attributes(vec![attr("method", "instantiate")]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::AdminSetResourceWithSig(msg) => {
            admin_set_resource_with_signature(deps, info, msg)
        }
        ExecuteMsg::ExecProposalWithSig(msg) => exec_proposal_with_signature(deps, env, msg),
    }
}

fn admin_set_resource_with_signature(
    mut deps: DepsMut,
    _info: MessageInfo,
    msg: SetResourceWithSigMsg,
) -> Result<Response, ContractError> {
    let mut state = STATE.load(deps.storage)?;

    // Validations
    let mut data: Vec<u8> = Vec::new();
    data.extend_from_slice(&msg.resource_id);
    data.extend_from_slice(&msg.function_sig);
    data.extend_from_slice(&msg.nonce.to_be_bytes());
    data.extend_from_slice(&msg.new_resource_id);
    data.extend_from_slice(msg.handler_addr.as_bytes());
    data.extend_from_slice(msg.execution_context_addr.as_bytes());

    if !signed_by_governor(deps.branch(), &data, &msg.sig, &state.governor)? {
        return Err(ContractError::Std(StdError::GenericErr {
            msg: "Invalid sig from governor".to_string(),
        }));
    }

    if msg.nonce <= state.proposal_nonce || state.proposal_nonce + 1048 < msg.nonce {
        return Err(ContractError::InvalidNonce);
    }

    // Save the info of "resource_id -> handler(contract)" in this contract.
    RESOURCEID2HANDLERADDR.save(deps.storage, &msg.new_resource_id, &msg.handler_addr)?;

    state.proposal_nonce = msg.nonce;
    STATE.save(deps.storage, &state)?;

    // Save the "resource" info in "handler" contract.
    let msgs: Vec<CosmosMsg> = vec![CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: msg.handler_addr,
        funds: vec![],
        msg: to_binary(&ExecutorExecMsg::SetResource {
            resource_id: msg.new_resource_id,
            contract_addr: msg.execution_context_addr,
        })
        .unwrap(),
    })];

    Ok(Response::new()
        .add_messages(msgs)
        .add_attributes(vec![attr("method", "admin_set_resource_with_sig")]))
}

fn exec_proposal_with_signature(
    mut deps: DepsMut,
    env: Env,
    msg: ExecProposalWithSigMsg,
) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;

    // Validations
    if !signed_by_governor(deps.branch(), &msg.data, &msg.sig, &state.governor)? {
        return Err(ContractError::Std(StdError::GenericErr {
            msg: "Invalid sig from governor".to_string(),
        }));
    }

    // Parse resourceID from the data
    let resource_id_bytes = &msg.data[0..32];
    let resource_id = element_encoder(resource_id_bytes);

    // Parse chain ID + chain type from the resource ID
    let execution_chain_id_type: u64 = get_chain_id_type(&resource_id_bytes[26..32]);

    // Verify current chain matches chain ID from resource ID
    let chain_id = compute_chain_id(&env.block.chain_id);
    if compute_chain_id_type(chain_id.into(), &COSMOS_CHAIN_TYPE) != execution_chain_id_type {
        return Err(ContractError::Std(StdError::GenericErr {
            msg: "Executing on wrong chain".to_string(),
        }));
    }

    // Execute the "proposal" in "handler" contract
    let handler_addr = RESOURCEID2HANDLERADDR.load(deps.storage, &resource_id)?;
    let msgs: Vec<CosmosMsg> = vec![CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: handler_addr,
        funds: vec![],
        msg: to_binary(&ExecutorExecMsg::ExecuteProposal {
            resource_id,
            data: msg.data,
        })
        .unwrap(),
    })];

    Ok(Response::new()
        .add_messages(msgs)
        .add_attributes(vec![attr("method", "execute_proposal_with_sig")]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetState {} => to_binary(&get_state(deps)?),
    }
}

fn get_state(deps: Deps) -> StdResult<StateResponse> {
    let state = STATE.load(deps.storage)?;
    Ok(StateResponse {
        governor: state.governor,
        proposal_nonce: state.proposal_nonce,
    })
}

// Verifying signature of governor over some datahash
fn signed_by_governor(
    deps: DepsMut,
    data: &[u8],
    sig: &[u8],
    governor: &[u8],
) -> Result<bool, ContractError> {
    let hashed_data = keccak_256(data);
    let verify_result = deps.api.secp256k1_verify(&hashed_data, sig, governor);

    verify_result.map_err(|e| ContractError::Std(StdError::VerificationErr { source: e }))
}
