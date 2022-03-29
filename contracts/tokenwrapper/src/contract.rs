#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, coins, from_binary, to_binary, Addr, BankMsg, Binary, CosmosMsg, Deps, DepsMut, Env,
    MessageInfo, Response, StdError, StdResult, Uint128, WasmMsg,
};
use cw2::set_contract_version;

use cw20::{BalanceResponse, Cw20ExecuteMsg, Cw20QueryMsg, Cw20ReceiveMsg, TokenInfoResponse};
use cw20_base::allowances::{
    execute_burn_from, execute_decrease_allowance, execute_increase_allowance, execute_send_from,
    execute_transfer_from, query_allowance,
};
use cw20_base::contract::{
    execute_burn, execute_mint, execute_send, execute_transfer, query_balance, query_token_info,
};

use cw20_base::state::{MinterData, TokenInfo, TOKEN_INFO};

use protocol_cosmwasm::error::ContractError;
use protocol_cosmwasm::token_wrapper::{Cw20HookMsg, ExecuteMsg, InstantiateMsg, QueryMsg};

use crate::state::{Supply, TOTAL_SUPPLY};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cosmwasm-tokenwrapper";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
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

    // set supply to 0
    let supply = Supply::default();
    TOTAL_SUPPLY.save(deps.storage, &supply)?;

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
        ExecuteMsg::Wrap {} => wrap_native(deps, env, info),

        ExecuteMsg::Unwrap { token, amount } => match token {
            Some(token) => unwrap_cw20(deps, env, info, token, amount),
            None => unwrap_native(deps, env, info, amount),
        },

        ExecuteMsg::Receive(msg) => receive_cw20(deps, env, info, msg),

        // these all come from cw20-base to implement the cw20 standard
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

fn wrap_native(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    // Check if the UST is sent.
    let sent_uusd = info
        .funds
        .iter()
        .find(|token| token.denom == *"uusd".to_string())
        .ok_or(ContractError::InsufficientFunds {})?;
    let to_mint = sent_uusd.amount;

    let mut supply = TOTAL_SUPPLY.load(deps.storage)?;
    supply.issued += to_mint;
    TOTAL_SUPPLY.save(deps.storage, &supply)?;

    // call into cw20-base to mint the token, call as self as no one else is allowed
    let sub_info = MessageInfo {
        sender: env.contract.address.clone(),
        funds: vec![],
    };
    execute_mint(deps, env, sub_info, info.sender.to_string(), to_mint)?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "wrap_native"),
        attr("from", info.sender),
        attr("minted", to_mint),
    ]))
}

fn unwrap_native(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    // Validate the "amount"
    if !is_valid_amount(deps.branch(), info.clone(), amount) {
        return Err(ContractError::Std(StdError::GenericErr {
            msg: "Invalid amount".to_string(),
        }));
    }

    // Calculate the remainder
    let total_supply = TOTAL_SUPPLY.load(deps.storage)?;
    let remainder = total_supply.issued - amount;

    // burn from the original caller
    execute_burn(deps.branch(), env, info.clone(), amount)?;

    // Save the "total_supply"
    TOTAL_SUPPLY.save(deps.storage, &Supply { issued: remainder })?;

    // Refund the native token(UST)
    let msgs: Vec<CosmosMsg> = vec![CosmosMsg::Bank(BankMsg::Send {
        to_address: info.sender.to_string(),
        amount: coins(amount.u128(), "uusd"),
    })];

    Ok(Response::new().add_messages(msgs).add_attributes(vec![
        attr("action", "unwrap_native"),
        attr("from", info.sender),
        attr("unwrap", amount),
        attr("refund", amount),
    ]))
}

fn unwrap_cw20(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token: Addr,
    amount: Uint128,
) -> Result<Response, ContractError> {
    // Validate the "token" address
    if !is_valid_address(deps.branch(), token.clone()) {
        return Err(ContractError::Std(StdError::GenericErr {
            msg: "Invalid Cw20 token address".to_string(),
        }));
    }

    // Validate the "token" amount
    if !is_valid_amount(deps.branch(), info.clone(), amount) {
        return Err(ContractError::Std(StdError::GenericErr {
            msg: "Invalid amount".to_string(),
        }));
    }

    // Calculate the remainder
    let total_supply = TOTAL_SUPPLY.load(deps.storage)?;
    let remainder = total_supply.issued - amount;

    // burn from the original caller
    execute_burn(deps.branch(), env, info.clone(), amount)?;

    // Save the "total_supply"
    TOTAL_SUPPLY.save(deps.storage, &Supply { issued: remainder })?;

    // Refund the Cw20 token
    let msgs: Vec<CosmosMsg> = vec![CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: token.to_string(),
        funds: vec![],
        msg: to_binary(&Cw20ExecuteMsg::Transfer {
            recipient: info.sender.to_string(),
            amount,
        })?,
    })];

    Ok(Response::new().add_messages(msgs).add_attributes(vec![
        attr("action", "unwrap_cw20"),
        attr("from", info.sender),
        attr("unwrap", amount),
        attr("refund", amount),
    ]))
}

fn receive_cw20(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    // Only Cw20 token contract can execute this message.
    if !is_valid_address(deps.branch(), info.sender) {
        return Err(ContractError::Unauthorized {});
    }

    let sender = cw20_msg.sender;
    let to_mint = cw20_msg.amount;
    match from_binary(&cw20_msg.msg) {
        Ok(Cw20HookMsg::Wrap {}) => {
            // Validate the token amount
            if to_mint.is_zero() {
                return Err(ContractError::Std(StdError::GenericErr {
                    msg: "Sent zero amount".to_string(),
                }));
            }

            let mut supply = TOTAL_SUPPLY.load(deps.storage)?;
            supply.issued += to_mint;
            TOTAL_SUPPLY.save(deps.storage, &supply)?;

            // call into cw20-base to mint the token, call as self as no one else is allowed
            let sub_info = MessageInfo {
                sender: env.contract.address.clone(),
                funds: vec![],
            };
            execute_mint(deps, env, sub_info, sender.to_string(), to_mint)?;

            Ok(Response::new().add_attributes(vec![
                attr("action", "wrap_cw20"),
                attr("from", sender),
                attr("minted", to_mint),
            ]))
        }
        Err(e) => Err(ContractError::Std(e)),
    }
}

fn is_valid_address(deps: DepsMut, token_address: Addr) -> bool {
    let token_info_query: StdResult<TokenInfoResponse> = deps
        .querier
        .query_wasm_smart(token_address, &Cw20QueryMsg::TokenInfo {});

    token_info_query.is_ok()
}

fn is_valid_amount(deps: DepsMut, info: MessageInfo, amount: Uint128) -> bool {
    let sender_token_balance = query_balance(deps.as_ref(), info.sender.to_string())
        .unwrap_or(BalanceResponse {
            balance: Uint128::zero(),
        })
        .balance;
    amount <= sender_token_balance
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        // TODO: Add custom queries.

        // inherited from cw20-base
        QueryMsg::TokenInfo {} => to_binary(&query_token_info(deps)?),
        QueryMsg::Balance { address } => to_binary(&query_balance(deps, address)?),
        QueryMsg::Allowance { owner, spender } => {
            to_binary(&query_allowance(deps, owner, spender)?)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coins, from_binary, Coin, Uint128};
    use cw20::{BalanceResponse, Cw20ReceiveMsg, TokenInfoResponse};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(&[]);

        let info = mock_info("creator", &[]);
        let instantiate_msg = InstantiateMsg {
            name: "Webb-WRAP".to_string(),
            symbol: "WWRP".to_string(),
            decimals: 6u8,
        };

        // We call ".unwrap()" to ensure succeed
        let res = instantiate(deps.as_mut(), mock_env(), info, instantiate_msg).unwrap();
        assert_eq!(res.messages.len(), 0);

        let query = query(deps.as_ref(), mock_env(), QueryMsg::TokenInfo {}).unwrap();
        let token_info_response: TokenInfoResponse = from_binary(&query).unwrap();

        assert_eq!(token_info_response.name, "Webb-WRAP".to_string());
        assert_eq!(token_info_response.symbol, "WWRP".to_string());
        assert_eq!(token_info_response.decimals, 6);
        assert_eq!(token_info_response.total_supply, Uint128::zero());
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
        };

        let _res = instantiate(deps.as_mut(), mock_env(), info, instantiate_msg).unwrap();

        // Try the wrapping the ust
        let info = mock_info("anyone", &coins(100, "uusd"));
        let wrap_msg = ExecuteMsg::Wrap {};
        let res = execute(deps.as_mut(), mock_env(), info, wrap_msg).unwrap();

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
            deps.as_ref(),
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
    fn test_unwrap_native() {
        let mut deps = mock_dependencies(&coins(100_u128, "uusd"));

        // Instantiate the tokenwrapper contract.
        let info = mock_info("creator", &[]);
        let instantiate_msg = InstantiateMsg {
            name: "Webb-WRAP".to_string(),
            symbol: "WWRP".to_string(),
            decimals: 6u8,
        };

        let _res = instantiate(deps.as_mut(), mock_env(), info, instantiate_msg).unwrap();

        // Try the wrapping the native token(UST)
        let info = mock_info("anyone", &coins(100, "uusd"));
        let wrap_msg = ExecuteMsg::Wrap {};
        let _res = execute(deps.as_mut(), mock_env(), info, wrap_msg).unwrap();

        // Try unwrapping the native token(UST)
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
        assert_eq!(token_balance.balance.u128(), 20);
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
                attr("minted", "100"),
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
        assert_eq!(token_balance.balance.u128(), 100);
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
        assert_eq!(token_balance.balance.u128(), 20);
    }
}
