use cosmwasm_std::testing::{
    mock_dependencies, mock_dependencies_with_balance, mock_env, mock_info, MockApi, MockQuerier,
    MockStorage,
};
use cosmwasm_std::{attr, coins, from_binary, to_binary, Addr, Coin, OwnedDeps, Uint128};
use cw20::{BalanceResponse, Cw20ReceiveMsg, TokenInfoResponse};

use protocol_cosmwasm::token_wrapper::{
    ConfigResponse, Cw20HookMsg, ExecuteMsg, FeeFromAmountResponse, GetAmountToWrapResponse,
    InstantiateMsg, QueryMsg,
};

use crate::contract::{execute, instantiate, query};

const NAME: &str = "Webb-WRAP";
const SYMBOL: &str = "WWRP";
const DECIMALS: u8 = 6;
const FEE_RECIPIENT: &str = "terra1qca9hs2qk2w29gqduaq9k720k9293qt7q8nszl";
const FEE_PERCENTAGE: u8 = 1_u8;
const NATIVE_TOKEN_DENOM: &str = "uusd";
const CW20_TOKEN: &str = "cw20_token";
const WRAPPING_LIMIT: u128 = 5000000;
const IS_NATIVE_ALLOWED: u32 = 1;

fn init_tokenwrapper(coins: Vec<Coin>) -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
    let mut deps = mock_dependencies_with_balance(&coins);

    let info = mock_info("creator", &[]);
    let instantiate_msg = InstantiateMsg {
        name: NAME.to_string(),
        symbol: SYMBOL.to_string(),
        decimals: DECIMALS,
        governor: None,
        fee_recipient: FEE_RECIPIENT.to_string(),
        fee_percentage: FEE_PERCENTAGE,
        native_token_denom: NATIVE_TOKEN_DENOM.to_string(),
        is_native_allowed: IS_NATIVE_ALLOWED,
        wrapping_limit: Uint128::from(WRAPPING_LIMIT),
    };

    // We call ".unwrap()" to ensure succeed
    let _ = instantiate(deps.as_mut(), mock_env(), info, instantiate_msg).unwrap();
    deps
}

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies();

    let info = mock_info("creator", &[]);
    let instantiate_msg = InstantiateMsg {
        name: NAME.to_string(),
        symbol: SYMBOL.to_string(),
        decimals: DECIMALS,
        governor: None,
        fee_recipient: FEE_RECIPIENT.to_string(),
        fee_percentage: FEE_PERCENTAGE,
        native_token_denom: NATIVE_TOKEN_DENOM.to_string(),
        is_native_allowed: IS_NATIVE_ALLOWED,
        wrapping_limit: Uint128::from(WRAPPING_LIMIT),
    };

    // We call ".unwrap()" to ensure succeed
    let res = instantiate(deps.as_mut(), mock_env(), info, instantiate_msg).unwrap();
    assert_eq!(res.messages.len(), 0);

    let query_bin = query(deps.as_ref(), mock_env(), QueryMsg::TokenInfo {}).unwrap();
    let token_info_response: TokenInfoResponse = from_binary(&query_bin).unwrap();

    assert_eq!(token_info_response.name, NAME.to_string());
    assert_eq!(token_info_response.symbol, SYMBOL.to_string());
    assert_eq!(token_info_response.decimals, 6);
    assert_eq!(token_info_response.total_supply, Uint128::zero());

    // Check the "config"
    let query_bin = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config_response: ConfigResponse = from_binary(&query_bin).unwrap();

    assert_eq!(config_response.governor, "creator".to_string());
    assert_eq!(
        config_response.native_token_denom,
        NATIVE_TOKEN_DENOM.to_string()
    );
    assert_eq!(config_response.fee_recipient, FEE_RECIPIENT.to_string());
    assert_eq!(config_response.fee_percentage, "1".to_string());
}

#[test]
fn test_wrap_native() {
    let mut deps = init_tokenwrapper([].to_vec());

    // Try the wrapping the native token
    let info = mock_info("anyone", &coins(100, "uusd"));
    let wrap_msg = ExecuteMsg::Wrap {
        sender: Some("owner".to_string()),
        recipient: Some("recipient".to_string()),
    };
    let res = execute(deps.as_mut(), mock_env(), info, wrap_msg).unwrap();

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "wrap_native"),
            attr("from", "anyone"),
            attr("owner", "owner"),
            attr("to", "recipient"),
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
            address: "recipient".to_string(),
        },
    )
    .unwrap();
    let token_balance: BalanceResponse = from_binary(&query).unwrap();
    assert_eq!(token_balance.balance.u128(), 99);
}

#[test]
fn test_unwrap_native() {
    let ctx_coins = coins(100_u128, "uusd");
    let mut deps = init_tokenwrapper(ctx_coins);

    // Try the wrapping the native token
    let info = mock_info("anyone", &coins(100, "uusd"));
    let wrap_msg = ExecuteMsg::Wrap {
        sender: None,
        recipient: None,
    };
    let _res = execute(deps.as_mut(), mock_env(), info, wrap_msg).unwrap();

    // Try unwrapping the native token
    let info = mock_info("anyone", &[]);
    let unwrap_msg = ExecuteMsg::Unwrap {
        token: None,
        amount: Uint128::from(80_u128),
        sender: None,
        recipient: None,
    };
    let res = execute(deps.as_mut(), mock_env(), info, unwrap_msg).unwrap();

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
    let mut deps = init_tokenwrapper([].to_vec());

    // Add a cw20 address to wrapping list.
    let info = mock_info("creator", &[]);
    let add_token_msg = ExecuteMsg::AddCw20TokenAddr {
        token: CW20_TOKEN.to_string(),
        nonce: 1,
    };
    let _res = execute(deps.as_mut(), mock_env(), info, add_token_msg).unwrap();

    // Try the wrapping the cw20 token
    let info = mock_info(CW20_TOKEN, &[]);
    let wrap_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "anyone".to_string(),
        amount: Uint128::from(100_u128),
        msg: to_binary(&Cw20HookMsg::Wrap {
            sender: None,
            recipient: None,
        })
        .unwrap(),
    });
    let res = execute(deps.as_mut(), mock_env(), info, wrap_msg).unwrap();

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "wrap_cw20"),
            attr("from", "anyone"),
            attr("owner", "anyone"),
            attr("to", "anyone"),
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
    let mut deps = init_tokenwrapper([].to_vec());

    // Add a cw20 address to wrapping list.
    let info = mock_info("creator", &[]);
    let add_token_msg = ExecuteMsg::AddCw20TokenAddr {
        token: CW20_TOKEN.to_string(),
        nonce: 1,
    };
    let _res = execute(deps.as_mut(), mock_env(), info, add_token_msg).unwrap();

    // Try the wrapping the cw20 token
    let info = mock_info(CW20_TOKEN, &[]);
    let wrap_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "anyone".to_string(),
        amount: Uint128::from(100_u128),
        msg: to_binary(&Cw20HookMsg::Wrap {
            sender: None,
            recipient: None,
        })
        .unwrap(),
    });
    let _res = execute(deps.as_mut(), mock_env(), info, wrap_msg).unwrap();

    // Try unwrapping the cw20 token
    let info = mock_info("anyone", &[]);
    let unwrap_msg = ExecuteMsg::Unwrap {
        token: Some(Addr::unchecked(CW20_TOKEN.to_string())),
        amount: Uint128::from(80_u128),
        sender: None,
        recipient: None,
    };
    let res = execute(deps.as_mut(), mock_env(), info, unwrap_msg).unwrap();

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "unwrap_cw20"),
            attr("from", "anyone"),
            attr("owner", "anyone"),
            attr("to", "anyone"),
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
    let deps = init_tokenwrapper([].to_vec());

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
    let deps = init_tokenwrapper([].to_vec());

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
fn test_update_config() {
    let mut deps = init_tokenwrapper([].to_vec());

    // Check the current governor.
    let query_bin = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config_response: ConfigResponse = from_binary(&query_bin).unwrap();
    assert_eq!(config_response.governor, "creator".to_string());

    // Update the config
    let info = mock_info("creator", &[]);
    let update_config_msg = ExecuteMsg::ConfigureGovernor {
        governor: Some("new_governor".to_string()),
    };

    let res = execute(deps.as_mut(), mock_env(), info, update_config_msg).unwrap();
    assert_eq!(res.attributes, vec![attr("method", "update_config"),]);

    // Check the new governor
    let query_bin = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config_response: ConfigResponse = from_binary(&query_bin).unwrap();
    assert_eq!(config_response.governor, "new_governor".to_string());
}

#[test]
fn test_add_token_addr() {
    let mut deps = init_tokenwrapper([].to_vec());

    // Add a new cw20 token addr.
    let info = mock_info("creator", &[]);
    let add_token_msg = ExecuteMsg::AddCw20TokenAddr {
        token: "new_cw20_token".to_string(),
        nonce: 1049,
    };

    // Failure since the invalid nonce value
    let err = execute(deps.as_mut(), mock_env(), info, add_token_msg).unwrap_err();
    assert_eq!(
        err.to_string(),
        "Nonce must be greater than current nonce. Nonce must not increment more than 1048"
            .to_string()
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

#[test]
fn test_remove_token_addr() {
    let mut deps = init_tokenwrapper([].to_vec());

    // Add a new cw20 token addr.
    let info = mock_info("creator", &[]);
    let add_token_msg = ExecuteMsg::AddCw20TokenAddr {
        token: "new_cw20_token".to_string(),
        nonce: 1,
    };
    let _res = execute(deps.as_mut(), mock_env(), info, add_token_msg).unwrap();

    // Remove a cw20 token address from wrapping list
    let info = mock_info("creator", &[]);
    let remove_token_msg = ExecuteMsg::RemoveCw20TokenAddr {
        token: "new_cw20_token".to_string(),
        nonce: 2,
    };

    let res = execute(deps.as_mut(), mock_env(), info, remove_token_msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("method", "remove_token"),
            attr("token", "new_cw20_token"),
        ]
    )
}
