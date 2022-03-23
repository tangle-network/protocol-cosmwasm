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
        pub fn new(vk_bytes: &[u8]) -> Self {
            Self {
                vk_bytes: vk_bytes.to_vec(),
            }
        }

        pub fn verify(&self, public_inp_bytes: Vec<u8>, proof_bytes: Vec<u8>) -> Result<bool> {
            ArkworksVerifierBn254::verify(&public_inp_bytes, &proof_bytes, &self.vk_bytes)
                .map_err(|_| Error::VerifierError)
        }
    }
}
