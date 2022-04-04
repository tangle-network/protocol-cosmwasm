use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{attr, coins, from_binary, to_binary, Addr, Coin, Uint128};
use cw20::{BalanceResponse, Cw20ReceiveMsg, TokenInfoResponse};

use protocol_cosmwasm::token_wrapper::{
    ConfigResponse, Cw20HookMsg, ExecuteMsg, FeeFromAmountResponse, GetAmountToWrapResponse,
    InstantiateMsg, QueryMsg,
};

use crate::contract::{execute, instantiate, query};

const FEE_RECIPIENT: &str = "terra1qca9hs2qk2w29gqduaq9k720k9293qt7q8nszl";
const FEE_PERCENTAGE: &str = "1";
const NATIVE_TOKEN_DENOM: &str = "uusd";

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies(&[]);

    let info = mock_info("creator", &[]);
    let instantiate_msg = InstantiateMsg {
        name: "Webb-WRAP".to_string(),
        symbol: "WWRP".to_string(),
        decimals: 6u8,
        governer: None,
        fee_recipient: FEE_RECIPIENT.to_string(),
        fee_percentage: FEE_PERCENTAGE.to_string(),
        native_token_denom: NATIVE_TOKEN_DENOM.to_string(),
        is_native_allowed: 1,
        wrapping_limit: "5000000".to_string(),
    };

    // We call ".unwrap()" to ensure succeed
    let res = instantiate(deps.as_mut(), mock_env(), info, instantiate_msg).unwrap();
    assert_eq!(res.messages.len(), 0);

    let query_bin = query(deps.as_ref(), mock_env(), QueryMsg::TokenInfo {}).unwrap();
    let token_info_response: TokenInfoResponse = from_binary(&query_bin).unwrap();

    assert_eq!(token_info_response.name, "Webb-WRAP".to_string());
    assert_eq!(token_info_response.symbol, "WWRP".to_string());
    assert_eq!(token_info_response.decimals, 6);
    assert_eq!(token_info_response.total_supply, Uint128::zero());

    // Check the "config"
    let query_bin = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config_response: ConfigResponse = from_binary(&query_bin).unwrap();

    assert_eq!(config_response.governer, "creator".to_string());
    assert_eq!(
        config_response.native_token_denom,
        NATIVE_TOKEN_DENOM.to_string()
    );
    assert_eq!(config_response.fee_recipient, FEE_RECIPIENT.to_string());
    assert_eq!(config_response.fee_percentage, "0.01".to_string());
}

#[test]
fn test_wrap_native() {
    let mut deps = mock_dependencies(&[]);

    // Instantiate the tokenwrapper contract.
    let info = mock_info("creator", &[]);
    let instantiate_msg = InstantiateMsg {
        name: "Webb-WRAP".to_string(),
        symbol: "WWRP".to_string(),
        decimals: 6u8,
        governer: None,
        fee_recipient: FEE_RECIPIENT.to_string(),
        fee_percentage: FEE_PERCENTAGE.to_string(),
        native_token_denom: NATIVE_TOKEN_DENOM.to_string(),
        is_native_allowed: 1,
        wrapping_limit: "5000000".to_string(),
    };

    let _res = instantiate(deps.as_mut(), mock_env(), info, instantiate_msg).unwrap();

    // Try the wrapping the native token
    let info = mock_info("anyone", &coins(100, "uusd"));
    let wrap_msg = ExecuteMsg::Wrap {};
    let res = execute(deps.as_mut(), mock_env(), info, wrap_msg).unwrap();

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "wrap_native"),
            attr("from", "anyone"),
            attr("minted", "99"),
            attr("fee", "1"),
        ]
    );

    assert_eq!(res.messages.len(), 1);

    // Check the "Webb_WRAP" token balance
    let query = query(
        deps.as_ref(),
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
fn test_unwrap_native() {
    let mut deps = mock_dependencies(&coins(100_u128, "uusd"));

    // Instantiate the tokenwrapper contract.
    let info = mock_info("creator", &[]);
    let instantiate_msg = InstantiateMsg {
        name: "Webb-WRAP".to_string(),
        symbol: "WWRP".to_string(),
        decimals: 6u8,
        governer: None,
        fee_recipient: FEE_RECIPIENT.to_string(),
        fee_percentage: FEE_PERCENTAGE.to_string(),
        native_token_denom: NATIVE_TOKEN_DENOM.to_string(),
        is_native_allowed: 1,
        wrapping_limit: "5000000".to_string(),
    };

    let _res = instantiate(deps.as_mut(), mock_env(), info, instantiate_msg).unwrap();

    // Try the wrapping the native token
    let info = mock_info("anyone", &coins(100, "uusd"));
    let wrap_msg = ExecuteMsg::Wrap {};
    let _res = execute(deps.as_mut(), mock_env(), info, wrap_msg).unwrap();

    // Try unwrapping the native token
    let info = mock_info("anyone", &[]);
    let unwrap_msg = ExecuteMsg::Unwrap {
        token: None,
        amount: Uint128::from(80_u128),
    };
    let res = execute(deps.as_mut(), mock_env(), info, unwrap_msg).unwrap();

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
        deps.as_ref(),
        mock_env(),
        QueryMsg::Balance {
            address: "anyone".to_string(),
        },
    )
    .unwrap();
    let token_balance: BalanceResponse = from_binary(&query).unwrap();
    assert_eq!(token_balance.balance.u128(), 19);
}

#[test]
fn test_wrap_cw20() {
    let cw20_address = "terra1fex9f78reuwhfsnc8sun6mz8rl9zwqh03fhwf3".to_string();
    let mut deps = crate::mock_querier::mock_dependencies(&[Coin {
        amount: Uint128::zero(),
        denom: cw20_address.clone(),
    }]);

    // Instantiate the tokenwrapper contract.
    let info = mock_info("creator", &[]);
    let instantiate_msg = InstantiateMsg {
        name: "Webb-WRAP".to_string(),
        symbol: "WWRP".to_string(),
        decimals: 6u8,
        governer: None,
        fee_recipient: FEE_RECIPIENT.to_string(),
        fee_percentage: FEE_PERCENTAGE.to_string(),
        native_token_denom: NATIVE_TOKEN_DENOM.to_string(),
        is_native_allowed: 1,
        wrapping_limit: "5000000".to_string(),
    };

    let _res = instantiate(deps.as_mut(), mock_env(), info, instantiate_msg).unwrap();

    // Try the wrapping the cw20 token
    let info = mock_info(&cw20_address, &[]);
    let wrap_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "anyone".to_string(),
        amount: Uint128::from(100_u128),
        msg: to_binary(&Cw20HookMsg::Wrap {}).unwrap(),
    });
    let res = execute(deps.as_mut(), mock_env(), info, wrap_msg).unwrap();

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "wrap_cw20"),
            attr("from", "anyone"),
            attr("minted", "99"),
            attr("fee", "1")
        ]
    );

    // Check the "Webb_WRAP" token balance
    let query = query(
        deps.as_ref(),
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
fn test_unwrap_cw20() {
    let cw20_address = "terra1fex9f78reuwhfsnc8sun6mz8rl9zwqh03fhwf3".to_string();
    let mut deps = crate::mock_querier::mock_dependencies(&[]);

    // Instantiate the tokenwrapper contract.
    let info = mock_info("creator", &[]);
    let instantiate_msg = InstantiateMsg {
        name: "Webb-WRAP".to_string(),
        symbol: "WWRP".to_string(),
        decimals: 6u8,
        governer: None,
        fee_recipient: FEE_RECIPIENT.to_string(),
        fee_percentage: FEE_PERCENTAGE.to_string(),
        native_token_denom: NATIVE_TOKEN_DENOM.to_string(),
        is_native_allowed: 1,
        wrapping_limit: "5000000".to_string(),
    };

    let _res = instantiate(deps.as_mut(), mock_env(), info, instantiate_msg).unwrap();

    // Try the wrapping the cw20 token
    let info = mock_info(&cw20_address, &[]);
    let wrap_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "anyone".to_string(),
        amount: Uint128::from(100_u128),
        msg: to_binary(&Cw20HookMsg::Wrap {}).unwrap(),
    });
    let _res = execute(deps.as_mut(), mock_env(), info, wrap_msg).unwrap();

    // Try unwrapping the cw20 token
    let info = mock_info("anyone", &[]);
    let unwrap_msg = ExecuteMsg::Unwrap {
        token: Some(Addr::unchecked(cw20_address)),
        amount: Uint128::from(80_u128),
    };
    let res = execute(deps.as_mut(), mock_env(), info, unwrap_msg).unwrap();

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "unwrap_cw20"),
            attr("from", "anyone"),
            attr("unwrap", "80"),
            attr("refund", "80"),
        ]
    );

    // Check the token amounts
    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Balance {
            address: "anyone".to_string(),
        },
    )
    .unwrap();
    let token_balance: BalanceResponse = from_binary(&res).unwrap();
    assert_eq!(token_balance.balance.u128(), 19);
}

#[test]
fn test_query_fee_from_wrap_amt() {
    let mut deps = crate::mock_querier::mock_dependencies(&[]);

    // Instantiate the tokenwrapper contract.
    let info = mock_info("creator", &[]);
    let instantiate_msg = InstantiateMsg {
        name: "Webb-WRAP".to_string(),
        symbol: "WWRP".to_string(),
        decimals: 6u8,
        governer: None,
        fee_recipient: FEE_RECIPIENT.to_string(),
        fee_percentage: FEE_PERCENTAGE.to_string(),
        native_token_denom: NATIVE_TOKEN_DENOM.to_string(),
        is_native_allowed: 1,
        wrapping_limit: "5000000".to_string(),
    };

    let _res = instantiate(deps.as_mut(), mock_env(), info, instantiate_msg).unwrap();

    // Check the query "query_fee_from_amount"
    let query_bin = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::FeeFromAmount {
            amount_to_wrap: "222".to_string(),
        },
    )
    .unwrap();
    let fee_response: FeeFromAmountResponse = from_binary(&query_bin).unwrap();
    assert_eq!(fee_response.amount_to_wrap, "222".to_string());
    assert_eq!(fee_response.fee_amt, "2".to_string());
}

#[test]
fn test_query_amt_to_wrap_from_target_amount() {
    let mut deps = crate::mock_querier::mock_dependencies(&[]);

    // Instantiate the tokenwrapper contract.
    let info = mock_info("creator", &[]);
    let instantiate_msg = InstantiateMsg {
        name: "Webb-WRAP".to_string(),
        symbol: "WWRP".to_string(),
        decimals: 6u8,
        governer: None,
        fee_recipient: FEE_RECIPIENT.to_string(),
        fee_percentage: FEE_PERCENTAGE.to_string(),
        native_token_denom: NATIVE_TOKEN_DENOM.to_string(),
        is_native_allowed: 1,
        wrapping_limit: "5000000".to_string(),
    };

    let _res = instantiate(deps.as_mut(), mock_env(), info, instantiate_msg).unwrap();

    // Check the query "query_fee_from_amount"
    let query_bin = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::GetAmountToWrap {
            target_amount: "222".to_string(),
        },
    )
    .unwrap();
    let fee_response: GetAmountToWrapResponse = from_binary(&query_bin).unwrap();
    assert_eq!(fee_response.target_amount, "222".to_string());
    assert_eq!(fee_response.amount_to_wrap, "224".to_string());
}

#[test]
fn test_set_governer() {
    let mut deps = mock_dependencies(&[]);

    let info = mock_info("creator", &[]);
    let instantiate_msg = InstantiateMsg {
        name: "Webb-WRAP".to_string(),
        symbol: "WWRP".to_string(),
        decimals: 6u8,
        governer: None,
        fee_recipient: FEE_RECIPIENT.to_string(),
        fee_percentage: FEE_PERCENTAGE.to_string(),
        native_token_denom: NATIVE_TOKEN_DENOM.to_string(),
        is_native_allowed: 1,
        wrapping_limit: "5000000".to_string(),
    };

    // We call ".unwrap()" to ensure succeed
    let _res = instantiate(deps.as_mut(), mock_env(), info, instantiate_msg).unwrap();

    // Check the current governer.
    let query_bin = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config_response: ConfigResponse = from_binary(&query_bin).unwrap();
    assert_eq!(config_response.governer, "creator".to_string());

    // Sets a new governer
    let info = mock_info("creator", &[]);
    let set_governer_msg = ExecuteMsg::SetGoverner {
        new_governer: "new_governer".to_string(),
    };

    let res = execute(deps.as_mut(), mock_env(), info, set_governer_msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("method", "set_governer"),
            attr("new_governer", "new_governer"),
        ]
    );
}

#[test]
fn test_set_native_allowed() {
    let mut deps = mock_dependencies(&[]);

    let info = mock_info("creator", &[]);
    let instantiate_msg = InstantiateMsg {
        name: "Webb-WRAP".to_string(),
        symbol: "WWRP".to_string(),
        decimals: 6u8,
        governer: None,
        fee_recipient: FEE_RECIPIENT.to_string(),
        fee_percentage: FEE_PERCENTAGE.to_string(),
        native_token_denom: NATIVE_TOKEN_DENOM.to_string(),
        is_native_allowed: 1,
        wrapping_limit: "5000000".to_string(),
    };

    // We call ".unwrap()" to ensure succeed
    let _res = instantiate(deps.as_mut(), mock_env(), info, instantiate_msg).unwrap();

    // Check the current "is_native_allowed".
    let query_bin = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config_response: ConfigResponse = from_binary(&query_bin).unwrap();
    assert_eq!(config_response.is_native_allowed, "true".to_string());

    // Sets "is_native_allowed"
    let info = mock_info("creator", &[]);
    let set_native_allowed_msg = ExecuteMsg::SetNativeAllowed {
        is_native_allowed: 0,
    };

    let res = execute(deps.as_mut(), mock_env(), info, set_native_allowed_msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("method", "set_native_allowed"),
            attr("is_native_allowed", "false"),
        ]
    );
}

#[test]
fn test_set_new_wrapping_limit() {
    let mut deps = mock_dependencies(&[]);

    let info = mock_info("creator", &[]);
    let instantiate_msg = InstantiateMsg {
        name: "Webb-WRAP".to_string(),
        symbol: "WWRP".to_string(),
        decimals: 6u8,
        governer: None,
        fee_recipient: FEE_RECIPIENT.to_string(),
        fee_percentage: FEE_PERCENTAGE.to_string(),
        native_token_denom: NATIVE_TOKEN_DENOM.to_string(),
        is_native_allowed: 1,
        wrapping_limit: "5000000".to_string(),
    };

    // We call ".unwrap()" to ensure succeed
    let _res = instantiate(deps.as_mut(), mock_env(), info, instantiate_msg).unwrap();

    // Check the current "wrapping_limit".
    let query_bin = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config_response: ConfigResponse = from_binary(&query_bin).unwrap();
    assert_eq!(config_response.wrapping_limit, "5000000".to_string());

    // Sets a new "wrapping_limit"
    let info = mock_info("creator", &[]);
    let set_new_limit_msg = ExecuteMsg::UpdateLimit {
        new_limit: "20000".to_string(),
    };

    let res = execute(deps.as_mut(), mock_env(), info, set_new_limit_msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("method", "set_wrapping_limit"),
            attr("new_limit", "20000"),
        ]
    );
}

#[test]
fn test_set_new_fee_perc() {
    let mut deps = mock_dependencies(&[]);

    let info = mock_info("creator", &[]);
    let instantiate_msg = InstantiateMsg {
        name: "Webb-WRAP".to_string(),
        symbol: "WWRP".to_string(),
        decimals: 6u8,
        governer: None,
        fee_recipient: FEE_RECIPIENT.to_string(),
        fee_percentage: FEE_PERCENTAGE.to_string(),
        native_token_denom: NATIVE_TOKEN_DENOM.to_string(),
        is_native_allowed: 1,
        wrapping_limit: "5000000".to_string(),
    };

    // We call ".unwrap()" to ensure succeed
    let _res = instantiate(deps.as_mut(), mock_env(), info, instantiate_msg).unwrap();

    // Check the current "fee_percentage".
    let query_bin = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config_response: ConfigResponse = from_binary(&query_bin).unwrap();
    assert_eq!(config_response.fee_percentage, "0.01".to_string());

    // Sets a new "fee_percentage"
    let info = mock_info("creator", &[]);
    let set_new_fee_perc_msg = ExecuteMsg::SetFee {
        new_fee_perc: "2".to_string(),
    };

    let res = execute(deps.as_mut(), mock_env(), info, set_new_fee_perc_msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![attr("method", "set_fee"), attr("new_fee_perc", "0.02"),]
    );
}

#[test]
fn test_set_new_fee_recipient() {
    let mut deps = mock_dependencies(&[]);

    let info = mock_info("creator", &[]);
    let instantiate_msg = InstantiateMsg {
        name: "Webb-WRAP".to_string(),
        symbol: "WWRP".to_string(),
        decimals: 6u8,
        governer: None,
        fee_recipient: FEE_RECIPIENT.to_string(),
        fee_percentage: FEE_PERCENTAGE.to_string(),
        native_token_denom: NATIVE_TOKEN_DENOM.to_string(),
        is_native_allowed: 1,
        wrapping_limit: "5000000".to_string(),
    };

    // We call ".unwrap()" to ensure succeed
    let _res = instantiate(deps.as_mut(), mock_env(), info, instantiate_msg).unwrap();

    // Check the current "fee_recipient".
    let query_bin = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config_response: ConfigResponse = from_binary(&query_bin).unwrap();
    assert_eq!(config_response.fee_recipient, FEE_RECIPIENT.to_string());

    // Sets a new "fee_recipient"
    let info = mock_info("creator", &[]);
    let set_new_fee_recpt_msg = ExecuteMsg::SetFeeRecipient {
        new_recipient: "new_recipient".to_string(),
    };

    let res = execute(deps.as_mut(), mock_env(), info, set_new_fee_recpt_msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("method", "set_fee_recipient"),
            attr("new_recipient", "new_recipient"),
        ]
    );
}

#[test]
fn test_add_token_addr() {
    let mut deps = mock_dependencies(&[]);

    let info = mock_info("creator", &[]);
    let instantiate_msg = InstantiateMsg {
        name: "Webb-WRAP".to_string(),
        symbol: "WWRP".to_string(),
        decimals: 6u8,
        governer: None,
        fee_recipient: FEE_RECIPIENT.to_string(),
        fee_percentage: FEE_PERCENTAGE.to_string(),
        native_token_denom: NATIVE_TOKEN_DENOM.to_string(),
        is_native_allowed: 1,
        wrapping_limit: "5000000".to_string(),
    };

    // We call ".unwrap()" to ensure succeed
    let _res = instantiate(deps.as_mut(), mock_env(), info, instantiate_msg).unwrap();

    // Add a new cw20 token addr.
    let info = mock_info("creator", &[]);
    let add_token_msg = ExecuteMsg::AddCw20TokenAddr {
        token: "new_cw20_token".to_string(),
        nonce: 2,
    };

    // Failure since the invalid nonce value
    let err = execute(deps.as_mut(), mock_env(), info, add_token_msg).unwrap_err();
    assert_eq!(
        err.to_string(),
        "Generic error: Nonce must increment by 1".to_string()
    );

    // Success
    let info = mock_info("creator", &[]);
    let add_token_msg = ExecuteMsg::AddCw20TokenAddr {
        token: "new_cw20_token".to_string(),
        nonce: 1,
    };

    let res = execute(deps.as_mut(), mock_env(), info, add_token_msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![attr("method", "add_token"), attr("token", "new_cw20_token"),]
    );
}
