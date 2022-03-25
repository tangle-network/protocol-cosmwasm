#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint128,
};
use cw2::set_contract_version;

use cw20_base::allowances::{
    execute_burn_from, execute_decrease_allowance, execute_increase_allowance, execute_send_from,
    execute_transfer_from, query_allowance,
};
use cw20_base::contract::{
    execute_burn, execute_mint, execute_send, execute_transfer, query_balance, query_token_info,
};
use cw20_base::msg::{QueryMsg as Cw20QueryMsg};
use cw20_base::state::{MinterData, TokenInfo, TOKEN_INFO};

use protocol_cosmwasm::error::ContractError;
use protocol_cosmwasm::token_wrapper::{ExecuteMsg, InstantiateMsg, QueryMsg};

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
        // TODO: Add the custom entries.

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

fn is_valid_address(deps: DepsMut, token_address: Addr) -> StdResult<bool> {
    let token_info_query: StdResult<TokenInfo> = deps
        .querier
        .query_wasm_smart(token_address, &Cw20QueryMsg::TokenInfo {});
    match token_info_query {
        Ok(_v) => Ok(true),
        Err(e) => Err(e),
    }
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
    use cosmwasm_std::{coins, from_binary};
    use cw20::TokenInfoResponse;

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
}
