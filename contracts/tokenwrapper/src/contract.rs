use std::str::FromStr;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, coins, from_binary, to_binary, Addr, BankMsg, Binary, CosmosMsg, Deps, DepsMut, Env,
    Fraction, MessageInfo, Response, StdError, StdResult, Uint128, WasmMsg,
};
use cw2::set_contract_version;

use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};
use cw20_base::allowances::{
    execute_burn_from, execute_decrease_allowance, execute_increase_allowance, execute_send_from,
    execute_transfer_from, query_allowance,
};
use cw20_base::contract::{
    execute_burn, execute_mint, execute_send, execute_transfer, query_balance, query_token_info,
};

use cw20_base::state::{MinterData, TokenInfo, TOKEN_INFO};

use protocol_cosmwasm::error::ContractError;
use protocol_cosmwasm::token_wrapper::{
    ConfigResponse, Cw20HookMsg, ExecuteMsg, FeeFromAmountResponse, GetAmountToWrapResponse,
    InstantiateMsg, QueryMsg, UpdateConfigMsg,
};

use crate::state::{Config, CONFIG, HISTORICAL_TOKENS, TOKENS};
use crate::utils::{
    calc_fee_perc_from_string, get_amount_to_wrap, get_fee_from_amount, is_valid_address,
    is_valid_unwrap_amount, is_valid_wrap_amount,
};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cosmwasm-tokenwrapper";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // store token info using cw20-base format
    let data = TokenInfo {
        name: msg.name,
        symbol: msg.symbol,
        decimals: msg.decimals,
        total_supply: Uint128::zero(),
        // set self as minter, so we can properly execute mint and burn
        mint: Some(MinterData {
            minter: env.contract.address,
            cap: None,
        }),
    };
    TOKEN_INFO.save(deps.storage, &data)?;

    // set config
    let governor = match msg.governor {
        Some(v) => deps.api.addr_validate(v.as_str())?,
        None => info.sender,
    };
    let fee_recipient = deps.api.addr_validate(msg.fee_recipient.as_str())?;
    let fee_percentage = calc_fee_perc_from_string(msg.fee_percentage)?;
    CONFIG.save(
        deps.storage,
        &Config {
            governor,
            fee_recipient,
            fee_percentage,
            native_token_denom: msg.native_token_denom,
            is_native_allowed: msg.is_native_allowed != 0,
            wrapping_limit: msg.wrapping_limit,
            proposal_nonce: 0_u64,
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
        /* -------  TokenWrapper functionality ------------ */
        // Used to wrap native tokens on behalf of a sender.
        ExecuteMsg::Wrap { sender, recipient } => wrap_native(deps, env, info, sender, recipient),

        // Used to unwrap native/cw20 tokens on behalf of a sender.
        ExecuteMsg::Unwrap {
            sender,
            token,
            amount,
            recipient,
        } => match token {
            // Unwrap the cw20 tokens.
            Some(token) => unwrap_cw20(deps, env, info, sender, token, amount, recipient),
            // Unwrap the native token.
            None => unwrap_native(deps, env, info, sender, amount, recipient),
        },

        // Used to wrap cw20 tokens on behalf of a sender.
        ExecuteMsg::Receive(msg) => wrap_cw20(deps, env, info, msg),
        /* ------------------------------------- */

        /* -----  Governance functionality ----- */
        // Resets the config. Only the governor can execute this entry.
        ExecuteMsg::UpdateConfig(msg) => update_config(deps, info, msg),

        // Add new cw20 token address to wrapping list
        ExecuteMsg::AddCw20TokenAddr { token, nonce } => add_token_addr(deps, info, token, nonce),

        // Remove cw20 token address from wrapping list (disallow wrapping)
        ExecuteMsg::RemoveCw20TokenAddr { token, nonce } => {
            remove_token_addr(deps, info, token, nonce)
        }
        /* --------------------------------------- */
        // These all come from cw20-base to implement the cw20 standard
        ExecuteMsg::Transfer { recipient, amount } => {
            Ok(execute_transfer(deps, env, info, recipient, amount)?)
        }
        ExecuteMsg::Burn { amount } => Ok(execute_burn(deps, env, info, amount)?),
        ExecuteMsg::Send {
            contract,
            amount,
            msg,
        } => Ok(execute_send(deps, env, info, contract, amount, msg)?),
        ExecuteMsg::IncreaseAllowance {
            spender,
            amount,
            expires,
        } => Ok(execute_increase_allowance(
            deps, env, info, spender, amount, expires,
        )?),
        ExecuteMsg::DecreaseAllowance {
            spender,
            amount,
            expires,
        } => Ok(execute_decrease_allowance(
            deps, env, info, spender, amount, expires,
        )?),
        ExecuteMsg::TransferFrom {
            owner,
            recipient,
            amount,
        } => Ok(execute_transfer_from(
            deps, env, info, owner, recipient, amount,
        )?),
        ExecuteMsg::BurnFrom { owner, amount } => {
            Ok(execute_burn_from(deps, env, info, owner, amount)?)
        }
        ExecuteMsg::SendFrom {
            owner,
            contract,
            amount,
            msg,
        } => Ok(execute_send_from(
            deps, env, info, owner, contract, amount, msg,
        )?),
    }
}

fn wrap_native(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    sender: Option<String>,
    recipient: Option<String>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // Validate the "is_native_allowed"
    if !config.is_native_allowed {
        return Err(ContractError::Std(StdError::GenericErr {
            msg: "Wrapping native token is not allowed in this token wrapper".to_string(),
        }));
    }

    // Validate the wrapping amount
    let wrapping_amount = info
        .funds
        .iter()
        .find(|token| token.denom == *config.native_token_denom)
        .ok_or(ContractError::InsufficientFunds {})?
        .amount;
    if wrapping_amount.is_zero() || !is_valid_wrap_amount(deps.branch(), wrapping_amount) {
        return Err(ContractError::Std(StdError::GenericErr {
            msg: "Invalid native token amount".to_string(),
        }));
    }

    // Calculate the "fee" & "amount_to_wrap".
    let cost_to_wrap =
        get_fee_from_amount(wrapping_amount, config.fee_percentage.numerator().u128());
    let left_over = wrapping_amount - cost_to_wrap;

    // call into cw20-base to mint the token, call as self as no one else is allowed
    let sub_info = MessageInfo {
        sender: env.contract.address.clone(),
        funds: vec![],
    };

    // Mint the wrapped tokens to "sender" address.
    let sender = sender.unwrap_or_else(|| info.sender.to_string());
    execute_mint(
        deps.branch(),
        env.clone(),
        sub_info,
        sender.clone(),
        left_over,
    )?;

    // Send the wrapped tokens to "recipient" address if any.
    if recipient.is_some() {
        let sub_info = MessageInfo {
            sender: deps.api.addr_validate(sender.as_str())?,
            funds: vec![],
        };
        execute_transfer(deps, env, sub_info, recipient.clone().unwrap(), left_over)?;
    }

    // send "fee" to fee_recipient
    let msgs: Vec<CosmosMsg> = vec![CosmosMsg::Bank(BankMsg::Send {
        to_address: config.fee_recipient.to_string(),
        amount: coins(cost_to_wrap.u128(), config.native_token_denom),
    })];

    Ok(Response::new().add_messages(msgs).add_attributes(vec![
        attr("action", "wrap_native"),
        attr("from", info.sender),
        attr("owner", sender.clone()),
        attr("to", recipient.unwrap_or(sender)),
        attr("minted", left_over),
        attr("fee", cost_to_wrap),
    ]))
}

fn unwrap_native(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    sender: Option<String>,
    amount: Uint128,
    recipient: Option<String>,
) -> Result<Response, ContractError> {
    let sender = sender.unwrap_or_else(|| info.sender.to_string());
    let config = CONFIG.load(deps.storage)?;
    // Validate the "is_native_allowed"
    if !config.is_native_allowed {
        return Err(ContractError::Std(StdError::GenericErr {
            msg: "Unwrapping native token is not allowed in this token wrapper".to_string(),
        }));
    }

    // Validate the "amount"
    if !is_valid_unwrap_amount(deps.branch(), &sender, amount) {
        return Err(ContractError::Std(StdError::GenericErr {
            msg: format!("Insufficient native token balance for sender({})", &sender),
        }));
    }

    // burn from the "sender"
    let sub_info = MessageInfo {
        sender: deps.api.addr_validate(sender.as_str())?,
        funds: vec![],
    };
    execute_burn(deps.branch(), env, sub_info, amount)?;

    // Send the native token to "recipient"
    let recipient = recipient.unwrap_or_else(|| sender.clone());
    let msgs: Vec<CosmosMsg> = vec![CosmosMsg::Bank(BankMsg::Send {
        to_address: recipient.clone(),
        amount: coins(amount.u128(), config.native_token_denom),
    })];

    Ok(Response::new().add_messages(msgs).add_attributes(vec![
        attr("action", "unwrap_native"),
        attr("from", info.sender),
        attr("owner", sender),
        attr("to", recipient),
        attr("unwrap", amount),
        attr("refund", amount),
    ]))
}

fn unwrap_cw20(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    sender: Option<String>,
    token: Addr,
    amount: Uint128,
    recipient: Option<String>,
) -> Result<Response, ContractError> {
    let sender = sender.unwrap_or_else(|| info.sender.to_string());

    // Validate the "token" address
    let is_valid_unwrap_address = HISTORICAL_TOKENS.has(deps.storage, token.clone());
    if !is_valid_unwrap_address {
        return Err(ContractError::Std(StdError::GenericErr {
            msg: "Invalid Cw20 token address".to_string(),
        }));
    }

    // Validate the "token" amount
    if !is_valid_unwrap_amount(deps.branch(), &sender, amount) {
        return Err(ContractError::Std(StdError::GenericErr {
            msg: "Insufficient cw20 token amount".to_string(),
        }));
    }

    // burn from the "sender"
    let sub_info = MessageInfo {
        sender: deps.api.addr_validate(sender.as_str())?,
        funds: vec![],
    };
    execute_burn(deps.branch(), env, sub_info, amount)?;

    // Send the Cw20 token to "recipient"
    let recipient = recipient.unwrap_or_else(|| sender.clone());
    let msgs: Vec<CosmosMsg> = vec![CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: token.to_string(),
        funds: vec![],
        msg: to_binary(&Cw20ExecuteMsg::Transfer {
            recipient: recipient.clone(),
            amount,
        })?,
    })];

    Ok(Response::new().add_messages(msgs).add_attributes(vec![
        attr("action", "unwrap_cw20"),
        attr("from", info.sender),
        attr("owner", sender),
        attr("to", recipient),
        attr("unwrap", amount),
        attr("refund", amount),
    ]))
}

fn wrap_cw20(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let cw20_address = info.sender;

    // Validate the cw20 address
    if !is_valid_address(deps.branch(), cw20_address.clone()) {
        return Err(ContractError::Std(StdError::GenericErr {
            msg: "Invalid Cw20 token address".to_string(),
        }));
    }

    // Validate the cw20 token amount.
    let cw20_token_amount = cw20_msg.amount;
    if cw20_token_amount.is_zero() || !is_valid_wrap_amount(deps.branch(), cw20_token_amount) {
        return Err(ContractError::Std(StdError::GenericErr {
            msg: "Invalid cw20 token".to_string(),
        }));
    }

    // Calculate the "fee" & "amount_to_wrap".
    let config = CONFIG.load(deps.storage)?;
    let cost_to_wrap =
        get_fee_from_amount(cw20_msg.amount, config.fee_percentage.numerator().u128());
    let left_over = cw20_msg.amount - cost_to_wrap;

    match from_binary(&cw20_msg.msg) {
        Ok(Cw20HookMsg::Wrap { sender, recipient }) => {
            // call into cw20-base to mint the token, call as self as no one else is allowed
            let sub_info = MessageInfo {
                sender: env.contract.address.clone(),
                funds: vec![],
            };
            let sender = sender.unwrap_or_else(|| cw20_msg.sender.clone());
            execute_mint(
                deps.branch(),
                env.clone(),
                sub_info,
                sender.clone(),
                left_over,
            )?;

            // Send the wrapped tokens to "recipient" address if any.
            if recipient.is_some() {
                let sub_info = MessageInfo {
                    sender: deps.api.addr_validate(sender.as_str())?,
                    funds: vec![],
                };
                execute_transfer(deps, env, sub_info, recipient.clone().unwrap(), left_over)?;
            }

            // Send the "fee" to "fee_recipient".
            let msgs: Vec<CosmosMsg> = vec![CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: cw20_address.to_string(),
                funds: vec![],
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: config.fee_recipient.to_string(),
                    amount: cost_to_wrap,
                })?,
            })];

            Ok(Response::new().add_messages(msgs).add_attributes(vec![
                attr("action", "wrap_cw20"),
                attr("from", cw20_msg.sender),
                attr("owner", sender.clone()),
                attr("to", recipient.unwrap_or(sender)),
                attr("minted", left_over),
                attr("fee", cost_to_wrap),
            ]))
        }
        Err(e) => Err(ContractError::Std(e)),
    }
}

fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    msg: UpdateConfigMsg,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    // Validate the tx sender.
    if config.governor != deps.api.addr_validate(info.sender.as_str())? {
        return Err(ContractError::Unauthorized {});
    }

    // Update the config
    if msg.governor.is_some() {
        config.governor = deps.api.addr_validate(msg.governor.unwrap().as_str())?;
    }

    if msg.is_native_allowed.is_some() {
        config.is_native_allowed = msg.is_native_allowed.unwrap();
    }

    if msg.wrapping_limit.is_some() {
        config.wrapping_limit = msg.wrapping_limit.unwrap_or(config.wrapping_limit);
    }

    if msg.fee_percentage.is_some() {
        config.fee_percentage = calc_fee_perc_from_string(msg.fee_percentage.unwrap())?;
    }

    if msg.fee_recipient.is_some() {
        config.fee_recipient = deps.api.addr_validate(&msg.fee_recipient.unwrap())?;
    }

    // Save the new config
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attributes(vec![attr("method", "update_config")]))
}

fn add_token_addr(
    deps: DepsMut,
    info: MessageInfo,
    token: String,
    nonce: u64,
) -> Result<Response, ContractError> {
    let token_addr = deps.api.addr_validate(token.as_str())?;
    if TOKENS.has(deps.storage, token_addr.clone()) {
        return Err(ContractError::Std(StdError::GenericErr {
            msg: "Token must not be valid".to_string(),
        }));
    }

    // Validate the tx sender.
    let mut config = CONFIG.load(deps.storage)?;
    if config.governor != deps.api.addr_validate(info.sender.as_str())? {
        return Err(ContractError::Unauthorized {});
    }

    // Validate the "nonce" value
    if nonce <= config.proposal_nonce || config.proposal_nonce + 1048 < nonce {
        return Err(ContractError::InvalidNonce);
    }

    // Add the "token" to wrapping list
    TOKENS.save(deps.storage, token_addr.clone(), &true)?;
    HISTORICAL_TOKENS.save(deps.storage, token_addr.clone(), &true)?;

    // Save the "proposal_nonce"
    config.proposal_nonce = nonce;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attributes(vec![
        attr("method", "add_token"),
        attr("token", token_addr.to_string()),
    ]))
}

fn remove_token_addr(
    deps: DepsMut,
    info: MessageInfo,
    token: String,
    nonce: u64,
) -> Result<Response, ContractError> {
    let token_addr = deps.api.addr_validate(token.as_str())?;
    let is_token_already_invalid = match TOKENS.load(deps.storage, token_addr.clone()) {
        Ok(v) => !v,
        Err(_) => return Err(ContractError::NotInitialized),
    };
    if is_token_already_invalid {
        return Err(ContractError::Std(StdError::GenericErr {
            msg: "Token must be valid".to_string(),
        }));
    }

    // Validate the tx sender.
    let mut config = CONFIG.load(deps.storage)?;
    if config.governor != deps.api.addr_validate(info.sender.as_str())? {
        return Err(ContractError::Unauthorized {});
    }

    // Validate the "nonce" value
    if nonce <= config.proposal_nonce || config.proposal_nonce + 1048 < nonce {
        return Err(ContractError::InvalidNonce);
    }

    // Remove the "token" from wrapping list
    TOKENS.save(deps.storage, token_addr.clone(), &false)?;

    // Save the "proposal_nonce"
    config.proposal_nonce = nonce;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attributes(vec![
        attr("method", "remove_token"),
        attr("token", token_addr.to_string()),
    ]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        // Query the "Config" state of the contract
        QueryMsg::Config {} => to_binary(&query_config(deps)?),

        // Query the fee amount, calculated from wrap amount
        QueryMsg::FeeFromAmount { amount_to_wrap } => {
            to_binary(&query_fee_from_amount(deps, amount_to_wrap)?)
        }

        // Query the real wrap amount, calculated from total amount
        QueryMsg::GetAmountToWrap { target_amount } => {
            to_binary(&query_amount_to_wrap(deps, target_amount)?)
        }

        // inherited from cw20-base
        QueryMsg::TokenInfo {} => to_binary(&query_token_info(deps)?),
        QueryMsg::Balance { address } => to_binary(&query_balance(deps, address)?),
        QueryMsg::Allowance { owner, spender } => {
            to_binary(&query_allowance(deps, owner, spender)?)
        }
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        governor: config.governor.to_string(),
        native_token_denom: config.native_token_denom,
        fee_recipient: config.fee_recipient.to_string(),
        fee_percentage: config.fee_percentage.to_string(),
        is_native_allowed: config.is_native_allowed.to_string(),
        wrapping_limit: config.wrapping_limit.to_string(),
        proposal_nonce: config.proposal_nonce.to_string(),
    })
}

fn query_fee_from_amount(deps: Deps, amount_to_wrap: String) -> StdResult<FeeFromAmountResponse> {
    let config = CONFIG.load(deps.storage)?;
    let amount_to_wrap = Uint128::from_str(&amount_to_wrap)?;
    let fee_perc = config.fee_percentage.numerator();
    let fee_amt = get_fee_from_amount(amount_to_wrap, fee_perc.u128());
    Ok(FeeFromAmountResponse {
        amount_to_wrap: amount_to_wrap.to_string(),
        fee_amt: fee_amt.to_string(),
    })
}

fn query_amount_to_wrap(deps: Deps, target_amount: String) -> StdResult<GetAmountToWrapResponse> {
    let config = CONFIG.load(deps.storage)?;
    let target_amount = Uint128::from_str(&target_amount)?;
    let fee_perc = config.fee_percentage.numerator();
    let amount_to_wrap = get_amount_to_wrap(target_amount, fee_perc.u128());
    Ok(GetAmountToWrapResponse {
        target_amount: target_amount.to_string(),
        amount_to_wrap: amount_to_wrap.to_string(),
    })
}
