use crate::keccak::Keccak256;
use cosmwasm_std::{StdError, Uint128};

/// Slice the length of the bytes array into 32bytes
pub fn element_encoder(v: &[u8]) -> [u8; 32] {
    let mut output = [0u8; 32];
    output.iter_mut().zip(v).for_each(|(b1, b2)| *b1 = *b2);
    output
}

/// Slice the length of bytes array into 4 bytes
pub fn bytes4_encoder(v: &[u8]) -> [u8; 4] {
    let mut output = [0u8; 4];
    output.iter_mut().zip(v).for_each(|(b1, b2)| *b1 = *b2);
    output
}

/// Computes the combination bytes of "chain_type" and "chain_id".
/// Combination rule: 8 bytes array(00 * 2 bytes + [chain_type] 2 bytes + [chain_id] 4 bytes)
/// Example:
///    chain_type - 0x0401, chain_id - 0x00000001 (big endian)
///    Result - [00, 00, 04, 01, 00, 00, 00, 01]
pub fn compute_chain_id_type(chain_id: u64, chain_type: &[u8]) -> u64 {
    let chain_id_value: u32 = chain_id.try_into().unwrap_or_default();
    let mut buf = [0u8; 8];
    #[allow(clippy::needless_borrow)]
    buf[2..4].copy_from_slice(&chain_type);
    buf[4..8].copy_from_slice(&chain_id_value.to_be_bytes());
    u64::from_be_bytes(buf)
}

/// Computes the numeric "chain_id" from string one.
/// This is only needed for Cosmos SDK blockchains since
/// their "chain_id"s are string(eg: "juno-1")
/// Rule:
///   1. Hash the "chain_id" to get 32-length bytes array
///       eg: keccak256("juno-1") => 4c22bf61f15534242ee9dba16dceb4c976851b1788680fb5ee2a7b568a294d21
///   2. Slice the last 4 bytes & convert it to `u32` numeric value
///       eg: 8a294d21(hex) -> 2317962529(decimal)
pub fn compute_chain_id(chain_id_str: &str) -> u32 {
    let hash_value = Keccak256::hash(chain_id_str.as_bytes()).expect("chain-id hashing error");
    let last_4_bytes = &hash_value[28..];

    let mut buf = [0u8; 4];
    buf[0..4].copy_from_slice(last_4_bytes);
    u32::from_be_bytes(buf)
}

/// Truncate and pad 256 bit slice
pub fn truncate_and_pad(t: &[u8]) -> Vec<u8> {
    let mut truncated_bytes = t[..20].to_vec();
    truncated_bytes.extend_from_slice(&[0u8; 12]);
    truncated_bytes
}

pub fn parse_string_to_uint128(v: String) -> Result<Uint128, StdError> {
    let res = match v.parse::<u128>() {
        Ok(v) => Uint128::from(v),
        Err(e) => return Err(StdError::GenericErr { msg: e.to_string() }),
    };
    Ok(res)
}

/// Get the `chain_id_type` from bytes array.
pub fn get_chain_id_type(chain_id_type: &[u8]) -> u64 {
    let mut buf = [0u8; 8];
    #[allow(clippy::needless_borrow)]
    buf[2..8].copy_from_slice(&chain_id_type);
    u64::from_be_bytes(buf)
}
