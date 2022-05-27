use cosmwasm_std::{Addr, Binary, Uint128};
use cw20::{Cw20ReceiveMsg, Expiration};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct InstantiateMsg {
    /// name of the Wrapping target token
    pub name: String,
    /// symbol / ticker of the Wrapping target token
    pub symbol: String,
    /// decimal places of the Wrapping target token (for UI)
    pub decimals: u8,

    /* --- Governance - related params --- */
    /// addr of governor
    pub governor: Option<String>,
    /// addr of fee recipient
    pub fee_recipient: String,
    /// fee_percentage( 0 ~ 100 )
    pub fee_percentage: String,
    /// native token denom string to be wrapped
    pub native_token_denom: String,
    /// flag of is_native_allowed
    pub is_native_allowed: u32,
    /// wrapping limit
    pub wrapping_limit: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /* ---    TokenWrapper functionality  ---- */
    /// Wrap the native token for "sender" address/tx sender address.
    Wrap {
        sender: Option<String>,
        recipient: Option<String>,
    },

    /// Unwrap the underlying tokens & re-send the fund(native or Cw20)
    Unwrap {
        sender: Option<String>,
        token: Option<Addr>,
        amount: Uint128,
        recipient: Option<String>,
    },

    /// Wrap the Cw20 token
    Receive(Cw20ReceiveMsg),

    /* ----------------------------------- */

    /* ---  Governance functionality  --- */
    /// Reset the config
    UpdateConfig(UpdateConfigMsg),

    /// Add cw20 token address to wrapping list
    AddCw20TokenAddr { token: String, nonce: u64 },

    /// Remove cw20 token address from wrapping list (disallow wrapping)
    RemoveCw20TokenAddr { token: String, nonce: u64 },

    /* ---------------------------------- */

    /* ---      Cw20 functions       --- */
    /// Implements CW20. Transfer is a base message to move tokens to another account without triggering actions
    Transfer { recipient: String, amount: Uint128 },
    /// Implements CW20. Burn is a base message to destroy tokens forever
    Burn { amount: Uint128 },
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
    BurnFrom { owner: String, amount: Uint128 },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw20HookMsg {
    /// Wrap Cw20 tokens
    Wrap {
        sender: Option<String>,
        recipient: Option<String>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct UpdateConfigMsg {
    pub governor: Option<String>,
    pub is_native_allowed: Option<bool>,
    pub wrapping_limit: Option<Uint128>,
    pub fee_percentage: Option<String>,
    pub fee_recipient: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct ConfigResponse {
    pub governor: String,
    pub native_token_denom: String,
    pub fee_recipient: String,
    pub fee_percentage: String,
    pub is_native_allowed: String,
    pub wrapping_limit: String,
    pub proposal_nonce: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct FeeFromAmountResponse {
    pub amount_to_wrap: String,
    pub fee_amt: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct GetAmountToWrapResponse {
    pub target_amount: String,
    pub amount_to_wrap: String,
}
