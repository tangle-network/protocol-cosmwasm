use cosmwasm_std::{Addr, Decimal, DepsMut, Fraction, StdError, Uint128};
use cw20::BalanceResponse;
use cw20_base::contract::{query_balance, query_token_info};
use protocol_cosmwasm::error::ContractError;

use crate::state::{CONFIG, TOKENS};

// Check if the cw20 token address is valid in "TOKENS".
pub fn is_valid_address(deps: DepsMut, token_address: Addr) -> bool {
    TOKENS.load(deps.storage, token_address).unwrap_or(false)
}

// Check if the "wrap_amount" is valid.
pub fn is_valid_wrap_amount(deps: DepsMut, amount: Uint128) -> bool {
    let total_supply = query_token_info(deps.as_ref()).unwrap().total_supply;
    let config = CONFIG.load(deps.storage).unwrap();
    amount
        .saturating_add(total_supply)
        .le(&config.wrapping_limit)
}

// Check if the "unwrap_amount" is valid.
pub fn is_valid_unwrap_amount(deps: DepsMut, sender: &str, amount: Uint128) -> bool {
    let sender_token_balance = query_balance(deps.as_ref(), sender.to_string())
        .unwrap_or(BalanceResponse {
            balance: Uint128::zero(),
        })
        .balance;
    amount <= sender_token_balance
}

// Calculates the "fee" from "wrap_amount".
pub fn get_fee_from_amount(amount_to_wrap: Uint128, fee_perc: u128) -> Uint128 {
    amount_to_wrap.multiply_ratio(fee_perc, Decimal::MAX.denominator())
}

// Calculate the "amount_to_send" from "deposit_target" amount.
pub fn get_amount_to_wrap(target_amount: Uint128, fee_perc: u128) -> Uint128 {
    target_amount.multiply_ratio(
        Decimal::MAX.denominator(),
        Decimal::MAX.denominator().u128() - fee_perc,
    )
}

pub fn calc_fee_perc_from_string(v: String) -> Result<Decimal, ContractError> {
    let res = match v.parse::<u64>() {
        Ok(v) => {
            if v > 100 {
                return Err(ContractError::Std(StdError::GenericErr {
                    msg: "Percentage should be in range [0, 100]".to_string(),
                }));
            } else {
                Decimal::percent(v)
            }
        }
        Err(e) => {
            return Err(ContractError::Std(StdError::GenericErr {
                msg: e.to_string(),
            }))
        }
    };
    Ok(res)
}
