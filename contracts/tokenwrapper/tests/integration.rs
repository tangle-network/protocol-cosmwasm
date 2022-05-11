//! This integration test tries to run and call the generated wasm.

use cosmwasm_std::{attr, from_binary, Coin, Response, Uint128};
use cosmwasm_vm::testing::{execute, instantiate, mock_env, mock_info, mock_instance, query};
use cw20::BalanceResponse;
use protocol_cosmwasm::token_wrapper::{ExecuteMsg, InstantiateMsg, QueryMsg};

// This line will test the output of cargo wasm
// static WASM: &[u8] = include_bytes!("../../../artifacts/cosmwasm_tokenwrapper.wasm");

// You can uncomment this line instead to test productionified build from rust-optimizer
// static WASM: &[u8] =
//     include_bytes!("../../../target/wasm32-unknown-unknown/release/cosmwasm_tokenwrapper.wasm");

// For the github CI, we copy the wasm file manually & import here.
static WASM: &[u8] = include_bytes!("./cosmwasm_tokenwrapper.wasm");

const NAME: &str = "Webb-WRAP";
const SYMBOL: &str = "WWRP";
const DECIMALS: u8 = 6;
const FEE_RECIPIENT: &str = "terra1qca9hs2qk2w29gqduaq9k720k9293qt7q8nszl";
const FEE_PERCENTAGE: &str = "1";
const NATIVE_TOKEN_DENOM: &str = "uusd";
const WRAPPING_LIMIT: u128 = 5000000;
const IS_NATIVE_ALLOWED: u32 = 1;

#[test]
fn integration_test_instantiate_tokenwrapper() {
    let mut deps = mock_instance(WASM, &[]);

    let msg = InstantiateMsg {
        name: NAME.to_string(),
        symbol: SYMBOL.to_string(),
        decimals: DECIMALS,
        governor: None,
        fee_recipient: FEE_RECIPIENT.to_string(),
        fee_percentage: FEE_PERCENTAGE.to_string(),
        native_token_denom: NATIVE_TOKEN_DENOM.to_string(),
        is_native_allowed: IS_NATIVE_ALLOWED,
        wrapping_limit: Uint128::from(WRAPPING_LIMIT),
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
        name: NAME.to_string(),
        symbol: SYMBOL.to_string(),
        decimals: DECIMALS,
        governor: None,
        fee_recipient: FEE_RECIPIENT.to_string(),
        fee_percentage: FEE_PERCENTAGE.to_string(),
        native_token_denom: NATIVE_TOKEN_DENOM.to_string(),
        is_native_allowed: IS_NATIVE_ALLOWED,
        wrapping_limit: Uint128::from(WRAPPING_LIMIT),
    };

    let info = mock_info("creator", &[]);
    let _res: Response = instantiate(&mut deps, mock_env(), info, msg).unwrap();

    // Wrap the native token
    let info = mock_info("anyone", &[Coin::new(100_u128, "uusd")]);
    let wrap_msg = ExecuteMsg::Wrap {
        sender: None,
        recipient: None,
    };
    let res: Response = execute(&mut deps, mock_env(), info, wrap_msg).unwrap();

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "wrap_native"),
            attr("from", "anyone"),
            attr("owner", "anyone"),
            attr("to", "anyone"),
            attr("minted", "99"),
            attr("fee", "1"),
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
    assert_eq!(token_balance.balance.u128(), 99);
}

#[test]
fn integration_test_tokenwrapper_unwrap_native() {
    let mut deps = mock_instance(WASM, &[]);

    // Instantiate the tokenwrapper
    let msg = InstantiateMsg {
        name: NAME.to_string(),
        symbol: SYMBOL.to_string(),
        decimals: DECIMALS,
        governor: None,
        fee_recipient: FEE_RECIPIENT.to_string(),
        fee_percentage: FEE_PERCENTAGE.to_string(),
        native_token_denom: NATIVE_TOKEN_DENOM.to_string(),
        is_native_allowed: IS_NATIVE_ALLOWED,
        wrapping_limit: Uint128::from(WRAPPING_LIMIT),
    };

    let info = mock_info("creator", &[]);
    let _res: Response = instantiate(&mut deps, mock_env(), info, msg).unwrap();

    // Wrap the native token
    let info = mock_info("anyone", &[Coin::new(100_u128, "uusd")]);
    let wrap_msg = ExecuteMsg::Wrap {
        sender: None,
        recipient: None,
    };
    let _res: Response = execute(&mut deps, mock_env(), info, wrap_msg).unwrap();

    // Unwrap the native token
    let info = mock_info("anyone", &[]);
    let unwrap_msg = ExecuteMsg::Unwrap {
        token: None,
        amount: Uint128::from(80_u128),
        sender: None,
        recipient: None,
    };
    let res: Response = execute(&mut deps, mock_env(), info, unwrap_msg).unwrap();

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "unwrap_native"),
            attr("from", "anyone"),
            attr("owner", "anyone"),
            attr("to", "anyone"),
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
    assert_eq!(token_balance.balance.u128(), 19);
}
