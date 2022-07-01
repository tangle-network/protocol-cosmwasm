use cosmwasm_std::{Addr, DepsMut, Uint128};
use cw20::BalanceResponse;
use cw20_base::contract::{query_balance, query_token_info};

use crate::state::{CONFIG, TOKENS};
use protocol_cosmwasm::token_wrapper::WRAP_FEE_CALC_DENOMINATOR;

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
pub fn get_fee_from_amount(amount_to_wrap: Uint128, fee_perc: u8) -> Uint128 {
    amount_to_wrap.multiply_ratio(fee_perc, WRAP_FEE_CALC_DENOMINATOR)
}

// Calculate the "amount_to_send" from "deposit_target" amount.
pub fn get_amount_to_wrap(target_amount: Uint128, fee_perc: u8) -> Uint128 {
    target_amount.multiply_ratio(
        WRAP_FEE_CALC_DENOMINATOR,
        WRAP_FEE_CALC_DENOMINATOR - fee_perc,
    )
}
