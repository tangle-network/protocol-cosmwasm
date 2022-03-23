pub use self::vanchor_verifier::VAnchorVerifier;

#[allow(clippy::all)]
pub mod vanchor_verifier {
    use crate::verifier::verifier::ArkworksVerifierBn254;
    use cosmwasm_std::{StdError, StdResult};
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    pub struct VAnchorVerifier {
        vk_bytes: Vec<u8>,
    }

    #[derive(Debug)]
    pub enum Error {
        /// Returned if error verifying
        VerifierError,
    }

    /// The verifier result type.
    pub type Result<T> = core::result::Result<T, Error>;

    impl VAnchorVerifier {
        pub fn new(max_edges: u32, ins: u32, outs: u32) -> StdResult<Self> {
            let vk_bytes: &[u8] = match (max_edges, ins, outs) {
                (2, 2, 2) =>  include_bytes!("../../../protocol-substrate-fixtures/vanchor/bn254/x5/2-2-2/verifying_key.bin"),
                (2, 16, 2) => include_bytes!("../../../protocol-substrate-fixtures/vanchor/bn254/x5/2-16-2/verifying_key.bin"),
                (32, 2, 2) =>  include_bytes!("../../../protocol-substrate-fixtures/vanchor/bn254/x5/32-2-2/verifying_key.bin"),
                (32, 16, 2) => include_bytes!("../../../protocol-substrate-fixtures/vanchor/bn254/x5/32-16-2/verifying_key.bin"),
                _ => return Err(StdError::GenericErr { msg: format!("Invalid ({}, {}, {}) group", max_edges, ins, outs) }),
            };
            Ok(Self {
                vk_bytes: vk_bytes.to_vec(),
            })
        }

        pub fn verify(&self, public_inp_bytes: Vec<u8>, proof_bytes: Vec<u8>) -> Result<bool> {
            ArkworksVerifierBn254::verify(&public_inp_bytes, &proof_bytes, &self.vk_bytes)
                .map_err(|_| Error::VerifierError)
        }
    }
}
