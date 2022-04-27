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
use protocol_cosmwasm::keccak::Keccak256;
use protocol_cosmwasm::signature_bridge::{
    ExecProposalWithSigMsg, ExecuteMsg, InstantiateMsg, QueryMsg, SetResourceWithSigMsg,
    StateResponse,
};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cosmwasm-signature-bridge";
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

    // Set "state"
    let governor = deps.api.addr_validate(&msg.initial_governor)?;
    STATE.save(
        deps.storage,
        &State {
            governor,
            proposal_nonce: 0,
        },
    )?;

    Ok(Response::new().add_attributes(vec![
        attr("method", "instantiate"),
        attr("governor", msg.initial_governor),
    ]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::AdminSetResourceWithSig(msg) => {
            admin_set_resource_with_signature(deps, info, msg)
        }
        ExecuteMsg::ExecProposalWithSig(msg) => exec_proposal_with_signature(deps, info, msg),
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
    data.extend_from_slice(&element_encoder(&msg.nonce.to_le_bytes()));
    data.extend_from_slice(&msg.new_resource_id);
    data.extend_from_slice(msg.handler_addr.as_bytes());
    data.extend_from_slice(msg.execution_context_addr.as_bytes());

    if !signed_by_governor(deps.branch(), &data, &msg.sig, state.governor.as_str())? {
        return Err(ContractError::Std(StdError::GenericErr {
            msg: "Invalid sig from governor".to_string(),
        }));
    }

    if msg.nonce != state.proposal_nonce + 1 {
        return Err(ContractError::Std(StdError::GenericErr {
            msg: "Invalid nonce".to_string(),
        }));
    }

    let func_sig = Keccak256::hash(
        b"adminSetResourceWithSignature(bytes32,bytes4,uint32,bytes32,address,address,bytes)",
    )
    .map_err(|_| ContractError::HashError)?;
    if msg.function_sig != func_sig[0..4] {
        return Err(ContractError::Std(StdError::GenericErr {
            msg: "Invalid function signature".to_string(),
        }));
    }

    // Save the info of "resource_id -> handler(contract)" in this contract.
    RESOURCEID2HANDLERADDR.save(
        deps.storage,
        &msg.new_resource_id,
        &deps.api.addr_validate(&msg.handler_addr)?,
    )?;

    state.proposal_nonce = msg.nonce;
    STATE.save(deps.storage, &state)?;

    // Save the "resource" info in "handler" contract.
    let msgs: Vec<CosmosMsg> = vec![CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: msg.handler_addr,
        funds: vec![],
        msg: to_binary(&ExecutorExecMsg::SetResource {
            resource_id: msg.resource_id,
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
    _info: MessageInfo,
    msg: ExecProposalWithSigMsg,
) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;

    // Validations
    if !signed_by_governor(deps.branch(), &msg.data, &msg.sig, state.governor.as_str())? {
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
    //
    // NOTE:
    // This part is prone to future changes since the current implementation
    // is based on assumption that the `chain_id` is number.
    // In fact, the `chain_id` of Cosmos SDK blockchains is string, not number.
    // For example, the `chain_id` of Terra blockchain(mainnet) is `columbus-5`.
    // Eventually, it should replace the `MOCK_CHAIN_ID` with `chain_id` obtained
    // inside contract(here).
    if compute_chain_id_type(MOCK_CHAIN_ID, &COSMOS_CHAIN_TYPE) != execution_chain_id_type {
        return Err(ContractError::Std(StdError::GenericErr {
            msg: "executing on wrong chain".to_string(),
        }));
    }

    // Execute the "proposal" in "handler" contract
    let handler_addr = RESOURCEID2HANDLERADDR
        .load(deps.storage, &resource_id)?
        .to_string();
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
        governor: state.governor.to_string(),
        proposal_nonce: state.proposal_nonce,
    })
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

// Slice the length of the bytes array into 32bytes
fn element_encoder(v: &[u8]) -> [u8; 32] {
    let mut output = [0u8; 32];
    output.iter_mut().zip(v).for_each(|(b1, b2)| *b1 = *b2);
    output
}

// Get the `chain_id_type` from bytes array.
pub fn get_chain_id_type(chain_id_type: &[u8]) -> u64 {
    let mut buf = [0u8; 8];
    #[allow(clippy::needless_borrow)]
    buf[2..8].copy_from_slice(&chain_id_type);
    u64::from_be_bytes(buf)
}

// Computes the combination bytes of "chain_type" and "chain_id".
// Combination rule: 8 bytes array(00 * 2 bytes + [chain_type] 2 bytes + [chain_id] 4 bytes)
// Example:
//  chain_type - 0x0401, chain_id - 0x00000001 (big endian)
//  Result - [00, 00, 04, 01, 00, 00, 00, 01]
pub fn compute_chain_id_type(chain_id: u64, chain_type: &[u8]) -> u64 {
    let chain_id_value: u32 = chain_id.try_into().unwrap_or_default();
    let mut buf = [0u8; 8];
    #[allow(clippy::needless_borrow)]
    buf[2..4].copy_from_slice(&chain_type);
    buf[4..8].copy_from_slice(&chain_id_value.to_be_bytes());
    u64::from_be_bytes(buf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{attr, from_binary};
    use protocol_cosmwasm::signature_bridge::StateResponse;

    const GOVERNOR: &str = "governor";

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg {
            initial_governor: GOVERNOR.to_string(),
        };
        let info = mock_info("creator", &[]);

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
        assert_eq!(
            res.attributes,
            vec![
                attr("method", "instantiate"),
                attr("governor", GOVERNOR.to_string())
            ]
        );

        // it worked, let's query the state
        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetState {}).unwrap();
        let state: StateResponse = from_binary(&res).unwrap();
        assert_eq!(state.governor, GOVERNOR.to_string());
        assert_eq!(state.proposal_nonce, 0);
    }
}
