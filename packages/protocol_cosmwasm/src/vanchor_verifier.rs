pub use self::vanchor_verifier::VAnchorVerifier;

#[allow(clippy::all)]
pub mod vanchor_verifier {
    use crate::verifier::verifier::ArkworksVerifierBn254;
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
        pub fn new() -> Self {
            let vk_bytes = include_bytes!(
                "../../../protocol-substrate-fixtures/vanchor/bn254/x5/verifying_key.bin"
            );
            Self {
                vk_bytes: vk_bytes.to_vec(),
            }
        }

        pub fn verify(&self, public_inp_bytes: Vec<u8>, proof_bytes: Vec<u8>) -> Result<bool> {
            ArkworksVerifierBn254::verify(&public_inp_bytes, &proof_bytes, &self.vk_bytes)
                .map_err(|_| Error::VerifierError)
        }
    }

    impl Default for VAnchorVerifier {
        fn default() -> Self {
            Self::new()
        }
    }
}
