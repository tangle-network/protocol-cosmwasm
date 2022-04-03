use cosmwasm_std::{Addr, Binary, Uint128};
use cw20::{Cw20ReceiveMsg, Expiration};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    /// name of the Wrapping target token
    pub name: String,
    /// symbol / ticker of the Wrapping target token
    pub symbol: String,
    /// decimal places of the Wrapping target token (for UI)
    pub decimals: u8,
    /// addr of governer
    pub governer: Option<String>,
    /// addr of fee recipient
    pub fee_recipient: String,
    /// fee_percentage( 0 ~ 100 )
    pub fee_percentage: String,
    /// native token denom string to be wrapped
    pub native_token_denom: String,
    /// flag of is_native_allowed
    pub is_native_allowed: u32,
    /// wrapping limit
    pub wrapping_limit: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Wrap the native tokens(UST).
    Wrap {},

    /// Unwrap the underlying tokens & re-send the fund(native or Cw20)
    Unwrap {
        token: Option<Addr>,
        amount: Uint128,
    },

    // Wrap the Cw20 token
    Receive(Cw20ReceiveMsg),

    /// Implements CW20. Transfer is a base message to move tokens to another account without triggering actions
    Transfer {
        recipient: String,
        amount: Uint128,
    },
    /// Implements CW20. Burn is a base message to destroy tokens forever
    Burn {
        amount: Uint128,
    },
    /// Implements CW20.  Send is a base message to transfer tokens to a contract and trigger an action
    /// on the receiving contract.
    Send {
        contract: String,
        amount: Uint128,
        msg: Binary,
    },
    /// Implements CW20 "approval" extension. Allows spender to access an additional amount tokens
    /// from the owner's (env.sender) account. If expires is Some(), overwrites current allowance
    /// expiration with this one.
    IncreaseAllowance {
        spender: String,
        amount: Uint128,
        expires: Option<Expiration>,
    },
    /// Implements CW20 "approval" extension. Lowers the spender's access of tokens
    /// from the owner's (env.sender) account by amount. If expires is Some(), overwrites current
    /// allowance expiration with this one.
    DecreaseAllowance {
        spender: String,
        amount: Uint128,
        expires: Option<Expiration>,
    },
    /// Implements CW20 "approval" extension. Transfers amount tokens from owner -> recipient
    /// if `env.sender` has sufficient pre-approval.
    TransferFrom {
        owner: String,
        recipient: String,
        amount: Uint128,
    },
    /// Implements CW20 "approval" extension. Sends amount tokens from owner -> contract
    /// if `env.sender` has sufficient pre-approval.
    SendFrom {
        owner: String,
        contract: String,
        amount: Uint128,
        msg: Binary,
    },
    /// Implements CW20 "approval" extension. Destroys tokens forever
    BurnFrom {
        owner: String,
        amount: Uint128,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Implements CW20. Returns the current balance of the given address, 0 if unset.
    Balance { address: String },
    /// Implements CW20. Returns metadata on the contract - name, decimals, supply, etc.
    TokenInfo {},
    /// Implements CW20 "allowance" extension.
    /// Returns how much spender can use from owner account, 0 if unset.
    Allowance { owner: String, spender: String },

    /// Custom queries
    /// Returns the Config of contract
    Config {},
    /// Calculates the "fee" from "amount_to_wrap"
    FeeFromAmount { amount_to_wrap: String },
    /// Calculates the "amount_to_wrap" for target amt
    GetAmountToWrap { target_amount: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw20HookMsg {
    /// Wrap Cw20 tokens
    Wrap {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct ConfigResponse {
    pub governer: String,
    pub native_token_denom: String,
    pub fee_recipient: String,
    pub fee_percentage: String,
    pub is_native_allowed: String,
    pub wrapping_limit: String,
    pub proposal_nonce: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct FeeFromAmountResponse {
    pub amount_to_wrap: String,
    pub fee_amt: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct GetAmountToWrapResponse {
    pub target_amount: String,
    pub amount_to_wrap: String,
}
