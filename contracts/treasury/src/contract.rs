#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, to_binary, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response,
    StdError, StdResult, Uint128, WasmMsg,
};
use cw2::set_contract_version;

use cw20::{BalanceResponse, Cw20ExecuteMsg, TokenInfoResponse};
use protocol_cosmwasm::error::ContractError;
use protocol_cosmwasm::treasury::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg};

use crate::state::{Config, CONFIG};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cosmwasm-treasury";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // set config
    let treasury_handler = deps.api.addr_validate(&msg.treasury_handler)?;
    CONFIG.save(
        deps.storage,
        &Config {
            treasury_handler,
            proposal_nonce: 0_u32,
        },
    )?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::RescueTokens {
            token_address,
            to,
            amount_to_rescue,
            nonce,
        } => rescue_tokens(deps, env, info, token_address, to, amount_to_rescue, nonce),
        ExecuteMsg::SetHandler { handler, nonce } => set_handler(deps, env, info, handler, nonce),
    }
}

fn rescue_tokens(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token_address: String,
    to: String,
    amount_to_rescue: Uint128,
    nonce: u32,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    let curr_handler = config.treasury_handler.clone();
    let proposal_nonce = config.proposal_nonce;

    // Validations
    if info.sender != curr_handler {
        return Err(ContractError::Unauthorized {});
    }
    if nonce <= proposal_nonce || proposal_nonce + 1048 < nonce {
        return Err(ContractError::InvalidNonce);
    }
    let to = deps.api.addr_validate(&to)?;
    if amount_to_rescue.is_zero() {
        return Err(ContractError::Std(StdError::GenericErr {
            msg: "Invalid rescue amount".to_string(),
        }));
    }

    let mut msgs: Vec<CosmosMsg> = vec![];

    // Validate the `token_address`.
    // Here, there are 2 possibilities for `token_address` - native token denom or CW20 token address.
    // For the validation, we try to query the `TokenInfo` from `token_address`.
    // If the `token_address` represents the native token denom, then the query returns error.
    // Otherwise, the query returns the result, which means it is CW20 token.
    let cw20_token_info_query: Result<TokenInfoResponse, StdError> = deps
        .querier
        .query_wasm_smart(token_address.to_string(), &cw20::Cw20QueryMsg::TokenInfo {});

    if cw20_token_info_query.is_err() {
        // Handle the case of the `token_address` is the native token denomination.
        let denom = token_address;
        let mut coin = deps
            .querier
            .query_balance(env.contract.address.to_string(), &denom)
            .unwrap_or(Coin {
                denom,
                amount: Uint128::zero(),
            });
        if !coin.amount.is_zero() {
            if coin.amount > amount_to_rescue {
                coin.amount = amount_to_rescue;
            }
            msgs.push(CosmosMsg::Bank(BankMsg::Send {
                to_address: to.to_string(),
                amount: vec![coin],
            }));
        }
    } else {
        // Handle the case of the `token_address` is the CW20 token address.
        let token_balance: BalanceResponse = deps.querier.query_wasm_smart(
            token_address.clone(),
            &cw20::Cw20QueryMsg::Balance {
                address: env.contract.address.to_string(),
            },
        )?;
        if !token_balance.balance.is_zero() {
            let amount = if token_balance.balance > amount_to_rescue {
                amount_to_rescue
            } else {
                token_balance.balance
            };
            msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: token_address,
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: to.to_string(),
                    amount,
                })
                .unwrap(),
                funds: vec![],
            }));
        }
    }

    config.proposal_nonce = nonce;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default().add_messages(msgs))
}

/// Sets a new handler for the contract
fn set_handler(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    handler: String,
    nonce: u32,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    let curr_handler = config.treasury_handler;
    let proposal_nonce = config.proposal_nonce;

    // Validations
    if info.sender != curr_handler {
        return Err(ContractError::Unauthorized {});
    }
    if nonce <= proposal_nonce || proposal_nonce + 1048 < nonce {
        return Err(ContractError::InvalidNonce);
    }

    // Save a new "handler"
    let new_handler = deps.api.addr_validate(&handler)?;
    config.treasury_handler = new_handler;
    config.proposal_nonce = nonce;

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "set_handler"),
        attr("handler", handler),
        attr("nonce", nonce.to_string()),
    ]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetConfig {} => to_binary(&query_config(deps)?),
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        treasury_handler: config.treasury_handler.to_string(),
        proposal_nonce: config.proposal_nonce.to_string(),
    })
}
