use cosmwasm_std::{Addr, Decimal, DepsMut, Fraction, MessageInfo, Uint128};
use cw20::BalanceResponse;
use cw20_base::contract::query_balance;

use crate::state::{CONFIG, TOKENS, TOTAL_SUPPLY};

// Check if the cw20 token address is valid in "TOKENS".
pub fn is_valid_address(deps: DepsMut, token_address: Addr) -> bool {
    TOKENS.load(deps.storage, token_address).unwrap_or(false)
}

// Check if the "wrap_amount" is valid.
pub fn is_valid_wrap_amount(deps: DepsMut, amount: Uint128) -> bool {
    let total_supply = TOTAL_SUPPLY.load(deps.storage).unwrap().issued;
    let config = CONFIG.load(deps.storage).unwrap();
    amount + total_supply <= config.wrapping_limit
}

// Check if the "unwrap_amount" is valid.
pub fn is_valid_unwrap_amount(deps: DepsMut, info: MessageInfo, amount: Uint128) -> bool {
    let sender_token_balance = query_balance(deps.as_ref(), info.sender.to_string())
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
        Decimal::MAX.denominator() - fee_perc,
    )
}
