use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
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
    DecodeError
}
