use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{attr, from_binary};

use protocol_cosmwasm::signature_bridge::StateResponse;
use protocol_cosmwasm::signature_bridge::{InstantiateMsg, QueryMsg};

use super::contract::{instantiate, query};

const GOVERNOR: [u8; 33] = [0u8; 33];

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies();

    let msg = InstantiateMsg {
        initial_governor: GOVERNOR.to_vec(),
    };
    let info = mock_info("creator", &[]);

    // we can just call .unwrap() to assert this was a success
    let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());
    assert_eq!(res.attributes, vec![attr("method", "instantiate"),]);

    // it worked, let's query the state
    let res = query(deps.as_ref(), mock_env(), QueryMsg::GetState {}).unwrap();
    let state: StateResponse = from_binary(&res).unwrap();
    assert_eq!(state.governor, GOVERNOR.to_vec());
    assert_eq!(state.proposal_nonce, 0);
}
