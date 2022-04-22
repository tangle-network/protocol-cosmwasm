#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{attr, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::set_contract_version;

use crate::state::{State, STATE};
use protocol_cosmwasm::error::ContractError;
use protocol_cosmwasm::signature_bridge::{ExecuteMsg, InstantiateMsg, QueryMsg, StateResponse};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cosmwasm-signature-bridge";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

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
        ExecuteMsg::AdminSetResWithSig => admin_set_res_with_signature(deps, info),
        ExecuteMsg::ExecProposalWithSig => exec_proposal_with_signature(deps, info),
    }
}

fn admin_set_res_with_signature(
    deps: DepsMut,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    // TODO
    Ok(Response::new())
}

fn exec_proposal_with_signature(
    deps: DepsMut,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    // TODO
    Ok(Response::new())
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
