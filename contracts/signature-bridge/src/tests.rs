use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{attr, from_binary};

use protocol_cosmwasm::signature_bridge::{ExecuteMsg, SetResourceWithSigMsg, StateResponse};
use protocol_cosmwasm::signature_bridge::{InstantiateMsg, QueryMsg};

use crate::contract::execute;

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

#[test]
fn test_admin_set_resource_with_sig() {
    let mut deps = mock_dependencies();

    let msg = InstantiateMsg {
        // initial_governor: vec![
        //     3, 208, 185, 115, 73, 181, 13, 228, 227, 43, 34, 172, 109, 175, 102, 189, 121, 191, 67,
        //     6, 0, 8, 47, 154, 46, 207, 188, 171, 180, 64, 237, 128, 170,
        // ],
        initial_governor: vec![4, 100,  81, 108,  94, 158, 255, 154, 81, 173,  90,   3,
        237,  45, 187, 171,   5, 154, 110,   3, 89, 216,  43, 238,
          0, 191, 152, 147, 130,   4, 165, 210, 76,   0, 172, 197,
        202, 223,   1, 104,  96, 144, 220, 239, 92,  28, 221, 115,
        114,  51,  99,  55, 153,  55, 190, 129, 34,  51,  55,  68,
         43,  61,  45,  99, 172],
    };
    let info = mock_info("creator", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    let execute_msg = SetResourceWithSigMsg {
        resource_id: [
            0, 0, 0, 0, 0, 0, 70, 127, 67, 208, 178, 211, 64, 89, 163, 174, 112, 212, 80, 84, 160,
            166, 140, 225, 211, 140, 4, 0, 223, 157, 27, 2,
        ],
        function_sig: [201, 68, 228, 8],
        nonce: 1,
        new_resource_id: [
            0, 0, 0, 0, 0, 0, 193, 232, 69, 11, 25, 122, 188, 167, 110, 15, 96, 17, 107, 206, 38,
            178, 0, 41, 59, 245, 4, 0, 223, 157, 27, 2,
        ],
        handler_addr: "juno1qyygux5t4s3a3l25k8psxjydhtudu5lnt0tk0szm8q4s27xa980sg5hcl0".to_string(),
        execution_context_addr: "juno1wdnc37yzmjvxksq89wdxwug0knuxau73ume3h6afngsmzsztsvwqefd7tg"
            .to_string(),
        // sig: vec![
        //     220, 106, 189, 116, 222, 186, 219, 233, 77, 125, 141, 125, 12, 1, 88, 147, 102, 53, 41,
        //     44, 65, 187, 245, 86, 151, 52, 196, 153, 2, 35, 124, 183, 44, 125, 235, 78, 195, 94,
        //     141, 251, 76, 242, 229, 169, 165, 37, 148, 131, 141, 41, 213, 236, 79, 10, 15, 71, 157,
        //     95, 29, 163, 73, 169, 180, 82,
        // ],
        sig: vec![ 
        160, 143, 101, 204,  89,  83,  45,  48,  60, 217, 148,
        91, 209, 239, 153, 149,  46, 209, 126, 221, 213, 255,
        96,  94,  14, 214, 205,  14,  57, 147,   3, 235,   1,
        30, 207, 190,  73, 210, 136,  14,  99, 213, 190,  62,
       144, 241, 163, 124,  81,  80,  21,   6,  78, 120, 213,
       162, 159, 137,  47, 249,  12,  20, 140, 116,  27],
    };

    let mut env = mock_env();
    env.block.chain_id = "testing".to_string();
    let res = execute(
        deps.as_mut(),
        env,
        mock_info("sender", &[]),
        ExecuteMsg::AdminSetResourceWithSig(execute_msg),
    )
    .unwrap();
    println!("res:{:?}", res);
}
