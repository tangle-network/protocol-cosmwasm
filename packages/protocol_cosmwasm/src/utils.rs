use cosmwasm_std::{StdError, Uint128};

// Computes the combination bytes of "chain_type" and "chain_id".
// Combination rule: 8 bytes array(00 * 2 bytes + [chain_type] 2 bytes + [chain_id] 4 bytes)
// Example:
//  chain_type - 0x0401, chain_id - 0x00000001 (big endian)
//  Result - [00, 00, 04, 01, 00, 00, 00, 01]
pub fn compute_chain_id_type(chain_id: u64, chain_type: &[u8]) -> u64 {
    let chain_id_value: u32 = chain_id.try_into().unwrap_or_default();
    let mut buf = [0u8; 8];
    #[allow(clippy::needless_borrow)]
    buf[2..4].copy_from_slice(&chain_type);
    buf[4..8].copy_from_slice(&chain_id_value.to_be_bytes());
    u64::from_be_bytes(buf)
}

// Truncate and pad 256 bit slice
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
