//! This integration test tries to run and call the generated wasm.

use cosmwasm_std::{attr, from_binary, to_binary, Coin, Response, Uint128, Uint256};
use cosmwasm_vm::testing::{
    execute, instantiate, mock_env, mock_info, mock_instance, query,
};
use cw20::{BalanceResponse, Cw20ReceiveMsg};
use protocol_cosmwasm::token_wrapper::{InstantiateMsg, ExecuteMsg, Cw20HookMsg, QueryMsg};


// This line will test the output of cargo wasm
// static WASM: &[u8] = include_bytes!("../../../artifacts/cosmwasm_tokenwrapper.wasm");

// You can uncomment this line instead to test productionified build from rust-optimizer
// static WASM: &[u8] =
//     include_bytes!("../../../target/wasm32-unknown-unknown/release/cosmwasm_tokenwrapper.wasm");

// For the github CI, we copy the wasm file manually & import here.
static WASM: &[u8] = include_bytes!("./cosmwasm_tokenwrapper.wasm");

#[test]
fn integration_test_instantiate_tokenwrapper() {
    let mut deps = mock_instance(WASM, &[]);

    let msg = InstantiateMsg {
        name: "Webb-WRAP".to_string(),
        symbol: "WWRP".to_string(),
        decimals: 6u8,
    };

    let info = mock_info("creator", &[]);
    let response: Response = instantiate(&mut deps, mock_env(), info, msg).unwrap();

    assert_eq!(response.messages.len(), 0);
}

#[test]
fn integration_test_tokenwrapper_wrap_native() {
    let mut deps = mock_instance(WASM, &[]);

    // Instantiate the tokenwrapper
    let msg = InstantiateMsg {
        name: "Webb-WRAP".to_string(),
        symbol: "WWRP".to_string(),
        decimals: 6u8,
    };

    let info = mock_info("creator", &[]);
    let _res: Response = instantiate(&mut deps, mock_env(), info, msg).unwrap();

    // Wrap the native token
    let info = mock_info("anyone", &[Coin::new(100_u128, "uusd")]);
    let wrap_msg = ExecuteMsg::Wrap {};
    let res: Response = execute(&mut deps, mock_env(), info, wrap_msg).unwrap();

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "wrap_native"),
            attr("from", "anyone"),
            attr("minted", "100"),
        ]
    );

     // Check the "Webb_WRAP" token balance
     let query = query(
        &mut deps,
        mock_env(),
        QueryMsg::Balance {
            address: "anyone".to_string(),
        },
    )
    .unwrap();
    let token_balance: BalanceResponse = from_binary(&query).unwrap();
    assert_eq!(token_balance.balance.u128(), 100);
}

#[test]
fn integration_test_tokenwrapper_unwrap_native() {
    let mut deps = mock_instance(WASM, &[]);

    // Instantiate the tokenwrapper
    let msg = InstantiateMsg {
        name: "Webb-WRAP".to_string(),
        symbol: "WWRP".to_string(),
        decimals: 6u8,
    };

    let info = mock_info("creator", &[]);
    let _res: Response = instantiate(&mut deps, mock_env(), info, msg).unwrap();

    // Wrap the native token
    let info = mock_info("anyone", &[Coin::new(100_u128, "uusd")]);
    let wrap_msg = ExecuteMsg::Wrap {};
    let _res: Response = execute(&mut deps, mock_env(), info, wrap_msg).unwrap();

    // Unwrap the native token
    let info = mock_info("anyone", &[]);
    let unwrap_msg = ExecuteMsg::Unwrap {
        token: None,
        amount: Uint128::from(80_u128),
    };
    let res: Response = execute(&mut deps, mock_env(), info, unwrap_msg).unwrap();

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "unwrap_native"),
            attr("from", "anyone"),
            attr("unwrap", "80"),
            attr("refund", "80"),
        ]
    );

    // Check the token amounts
    let query = query(
        &mut deps,
        mock_env(),
        QueryMsg::Balance {
            address: "anyone".to_string(),
        },
    )
    .unwrap();
    let token_balance: BalanceResponse = from_binary(&query).unwrap();
    assert_eq!(token_balance.balance.u128(), 20);
}
