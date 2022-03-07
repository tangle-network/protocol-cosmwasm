pub use self::anchor_verifier::AnchorVerifier;

mod verifier {
    use ark_crypto_primitives::{Error, SNARK};
    use ark_ec::PairingEngine;
    use ark_groth16::{Groth16, Proof, VerifyingKey};
    use ark_serialize::CanonicalDeserialize;
    use ark_std::marker::PhantomData;
    use arkworks_utils::utils::to_field_elements;
    pub struct ArkworksVerifierGroth16<E: PairingEngine>(PhantomData<E>);

    pub fn verify_groth16<E: PairingEngine>(
        vk: &VerifyingKey<E>,
        public_inputs: &[E::Fr],
        proof: &Proof<E>,
    ) -> Result<bool, Error> {
        let res = Groth16::<E>::verify(vk, public_inputs, proof)?;
        Ok(res)
    }

    impl<E: PairingEngine> ArkworksVerifierGroth16<E> {
        pub fn verify(
            public_inp_bytes: &[u8],
            proof_bytes: &[u8],
            vk_bytes: &[u8],
        ) -> Result<bool, Error> {
            let public_input_field_elts = to_field_elements::<E::Fr>(public_inp_bytes)?;
            let vk = VerifyingKey::<E>::deserialize(vk_bytes)?;
            let proof = Proof::<E>::deserialize(proof_bytes)?;
            let res = verify_groth16::<E>(&vk, &public_input_field_elts, &proof)?;
            Ok(res)
        }
    }

    use ark_bn254::Bn254;
    pub type ArkworksVerifierBn254 = ArkworksVerifierGroth16<Bn254>;
}
#[allow(clippy::all)]
pub mod anchor_verifier {
    use super::verifier::{ArkworksVerifierBn254};
    use serde::{Serialize, Deserialize};

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
        pub fn new() -> Self {
            let vk_bytes = include_bytes!(
                "../../../protocol-substrate-fixtures/fixed-anchor/bn254/x5/verifying_key.bin"
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

    impl Default for AnchorVerifier {
        fn default() -> Self {
            Self::new()
        }
    }
}