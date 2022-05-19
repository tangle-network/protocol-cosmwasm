use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Unnecessary_funds")]
    UnnecessaryFunds {},

    #[error("Insufficient_funds")]
    InsufficientFunds {},
    // Add any other custom errors you like here.
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
    #[error("Invalid Cw20 Hook message")]
    InvalidCw20HookMsg,

    /* -------   mixer related error  ------- */
    /// Returned if the mixer is not initialized
    #[error("NotInitialized")]
    NotInitialized,
    /// Returned if the mixer is already initialized
    #[error("AlreadyInitialized")]
    AlreadyInitialized,
    /// Returned if the merkle tree is full.
    #[error("FullMerkleTree")]
    MerkleTreeIsFull,
    /// Hash error
    #[error("HashError")]
    HashError,
    /// Verify error
    #[error("VerifyError")]
    VerifyError,
    // Failing to decode a hex string
    #[error("DecodeError")]
    DecodeError,

    // Returned if a mapping item is not found
    #[error("Mapping item not found")]
    ItemNotFound,

    /*  ------ Anchor errors ------ */
    #[error("Invalid merkle roots")]
    InvaidMerkleRoots,

    #[error("Unknown root")]
    UnknownRoot,

    #[error("Invalid withdraw proof")]
    InvalidWithdrawProof,

    #[error("No anchor found")]
    NoAnchorFound,

    #[error("Invalid arbitrary data passed")]
    InvalidArbitraryData,

    #[error("Invalid nullifier that is already used")]
    AlreadyRevealedNullfier,

    #[error("Edge already exists")]
    EdgeAlreadyExists,

    #[error("Too many edges")]
    TooManyEdges,

    #[error("Invalid nonce")]
    InvalidNonce,

    /*  ------ VAnchor errors ------ */
    #[error("Invalid execution entry")]
    InvalidExecutionEntry,

    #[error("Invalid deposit amount")]
    InvalidDepositAmount,

    #[error("Invalid withdraw amount")]
    InvalidWithdrawAmount,

    #[error("Invalid ext data")]
    InvalidExtData,

    #[error("Invalid fee amount")]
    InvalidFeeAmount,

    #[error("Invalid ext amount")]
    InvalidExtAmount,

    #[error("Invalid public amount")]
    InvalidPublicAmount,

    #[error("Invalid transaction proof")]
    InvalidTxProof,

    /*  ------ TokenWrapper errors ------ */
    // For simplicity, it just converts all the cw20_base errors to Std error.
    #[error("Invalid CW20 token address")]
    InvalidCw20Token,
}

impl From<cw20_base::ContractError> for ContractError {
    fn from(err: cw20_base::ContractError) -> Self {
        match err {
            cw20_base::ContractError::Std(error) => ContractError::Std(error),
            cw20_base::ContractError::Unauthorized {}
            | cw20_base::ContractError::CannotSetOwnAccount {}
            | cw20_base::ContractError::InvalidZeroAmount {}
            | cw20_base::ContractError::Expired {}
            | cw20_base::ContractError::NoAllowance {}
            | cw20_base::ContractError::CannotExceedCap {}
            | cw20_base::ContractError::LogoTooBig {}
            | cw20_base::ContractError::InvalidPngHeader {}
            | cw20_base::ContractError::InvalidXmlPreamble {}
            | cw20_base::ContractError::DuplicateInitialBalanceAddresses {} => {
                ContractError::Std(StdError::generic_err(err.to_string()))
            }
        }
    }
}
