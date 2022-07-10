use cosmwasm_std::testing::{
    mock_dependencies, mock_dependencies_with_balance, mock_env, mock_info, MockApi, MockQuerier,
    MockStorage,
};
use cosmwasm_std::{attr, coin, from_binary, Coin, OwnedDeps, Uint128};

use protocol_cosmwasm::error::ContractError;
use protocol_cosmwasm::treasury::{
    ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg, TokenInfo,
};

use crate::contract::{execute, instantiate, query};

const TREASURY_HANDLER: &str = "treasury-handler";

fn create_treasury(coins: Vec<Coin>) -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
    let mut deps = if coins.is_empty() {
        mock_dependencies()
    } else {
        mock_dependencies_with_balance(&coins)
    };
    let init_msg = InstantiateMsg {
        treasury_handler: TREASURY_HANDLER.to_string(),
    };

    // Should pass this "unwrap" if success.
    let _res = instantiate(
        deps.as_mut(),
        mock_env(),
        mock_info("governor", &[]),
        init_msg,
    )
    .unwrap();
    deps
}

#[test]
fn test_treasury_proper_initialization() {
    let mut deps = mock_dependencies();
    let init_msg = InstantiateMsg {
        treasury_handler: TREASURY_HANDLER.to_string(),
    };

    // Should pass this "unwrap" if success.
    let _res = instantiate(
        deps.as_mut(),
        mock_env(),
        mock_info("governor", &[]),
        init_msg,
    )
    .unwrap();

    // Check the config
    let res = query(deps.as_ref(), mock_env(), QueryMsg::GetConfig {}).unwrap();
    let config_resp: ConfigResponse = from_binary(&res).unwrap();
    assert_eq!(config_resp.proposal_nonce, "0".to_string());
    assert_eq!(config_resp.treasury_handler, TREASURY_HANDLER.to_string());
}

#[test]
fn test_treasury_set_handler() {
    let new_handler: &str = "new-handler-address";
    let nonce: u32 = 2u32;

    let mut deps = create_treasury(vec![]);

    // Fails to "set handler" if tx sender is not current handler addr
    let info = mock_info("anyone", &[]);
    let set_handler_msg = ExecuteMsg::SetHandler {
        handler: new_handler.to_string(),
        nonce,
    };
    let err = execute(deps.as_mut(), mock_env(), info, set_handler_msg).unwrap_err();
    assert_eq!(err, ContractError::Unauthorized {});

    // Fails to "set handler" if "nonce" is too big or small
    let info = mock_info(TREASURY_HANDLER, &[]);
    let set_handler_msg = ExecuteMsg::SetHandler {
        handler: new_handler.to_string(),
        nonce: nonce + 2000,
    };
    let err = execute(deps.as_mut(), mock_env(), info, set_handler_msg).unwrap_err();
    assert_eq!(err, ContractError::InvalidNonce);

    // Succeed to "set handler"
    let info = mock_info(TREASURY_HANDLER, &[]);
    let set_handler_msg = ExecuteMsg::SetHandler {
        handler: new_handler.to_string(),
        nonce,
    };
    let res = execute(deps.as_mut(), mock_env(), info, set_handler_msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "set_handler"),
            attr("handler", new_handler),
            attr("nonce", nonce.to_string()),
        ]
    );
}

#[test]
fn test_treasury_rescue_tokens() {
    let nonce: u32 = 2u32;
    let to: String = "recipient".to_string();

    let mut deps = create_treasury(vec![coin(100_u128, "earth"), coin(100_u128, "moon")]);

    // Fails to "rescue tokens" since the caller is not "handler"
    let info = mock_info("anyone", &[]);
    let rescue_tokens_msg = ExecuteMsg::RescueTokens {
        token_info: TokenInfo::Native("earth".to_string()),
        amount_to_rescue: Uint128::from(200_u128),
        to: to.clone(),
        nonce,
    };
    let err = execute(deps.as_mut(), mock_env(), info, rescue_tokens_msg).unwrap_err();
    assert_eq!(err, ContractError::Unauthorized {});

    // Fails to "rescue tokens" if "nonce" is too big or small
    let info = mock_info(TREASURY_HANDLER, &[]);
    let rescue_tokens_msg = ExecuteMsg::RescueTokens {
        token_info: TokenInfo::Native("earth".to_string()),
        amount_to_rescue: Uint128::from(200_u128),
        to: to.clone(),
        nonce: nonce + 2000,
    };
    let err = execute(deps.as_mut(), mock_env(), info, rescue_tokens_msg).unwrap_err();
    assert_eq!(err, ContractError::InvalidNonce);

    // Fails to "rescue tokens" since "amount_to_rescue" is 0
    let info = mock_info(TREASURY_HANDLER, &[]);
    let rescue_tokens_msg = ExecuteMsg::RescueTokens {
        token_info: TokenInfo::Native("earth".to_string()),
        amount_to_rescue: Uint128::zero(),
        to: to.clone(),
        nonce,
    };
    let err = execute(deps.as_mut(), mock_env(), info, rescue_tokens_msg).unwrap_err();
    assert_eq!(
        err.to_string(),
        "Generic error: Invalid rescue amount".to_string()
    );

    // Succeed to "rescue tokens" since asking for zero amount tokens
    // But, it does not send any tokens
    let info = mock_info(TREASURY_HANDLER, &[]);
    let rescue_tokens_msg = ExecuteMsg::RescueTokens {
        token_info: TokenInfo::Native("sun".to_string()),
        amount_to_rescue: Uint128::from(100_u128),
        to: to.clone(),
        nonce,
    };
    let res = execute(deps.as_mut(), mock_env(), info, rescue_tokens_msg).unwrap();
    assert_eq!(res.messages.len(), 0);

    // Succeed to "rescue tokens"
    let info = mock_info(TREASURY_HANDLER, &[]);
    let rescue_token_msg = ExecuteMsg::RescueTokens {
        token_info: TokenInfo::Native("earth".to_string()),
        amount_to_rescue: Uint128::from(200_u128),
        nonce: nonce + 1,
        to,
    };
    let res = execute(deps.as_mut(), mock_env(), info, rescue_token_msg).unwrap();
    assert_eq!(res.messages.len(), 1);
}
