pub use self::anchor_verifier::AnchorVerifier;

#[allow(clippy::all)]
pub mod anchor_verifier {
    use crate::verifier::verifier::ArkworksVerifierBn254;
    use cosmwasm_std::{StdError, StdResult};
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    pub struct AnchorVerifier {
        vk_bytes: Vec<u8>,
    }

    #[derive(Debug)]
    pub enum Error {
        /// Returned if error verifying
        VerifierError,
    }

    /// The verifier result type.
    pub type Result<T> = core::result::Result<T, Error>;

    impl AnchorVerifier {
        pub fn new(max_edges: u32) -> StdResult<Self> {
            let vk_bytes: &[u8] = match max_edges {
                2 =>  include_bytes!("../../../substrate-fixtures/fixed-anchor/bn254/x5/2/verifying_key.bin"),
                16 => include_bytes!("../../../substrate-fixtures/fixed-anchor/bn254/x5/16/verifying_key.bin"),
                32 => include_bytes!("../../../substrate-fixtures/fixed-anchor/bn254/x5/32/verifying_key.bin"),
                _ => return Err( StdError::GenericErr { msg: "Invalid max_edges".to_string() } ),
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
