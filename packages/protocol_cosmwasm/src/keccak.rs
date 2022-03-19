pub use keccak::Keccak256;

mod hasher {
    use ark_ff::{BigInteger, PrimeField};
    use ark_std::{marker::PhantomData, vec::Vec};
    use arkworks_setups::common::keccak_256;

    pub struct Keccak256<F: PrimeField>(PhantomData<F>);

    impl<F: PrimeField> Keccak256<F> {
        pub fn hash(input: &[u8], _param_bytes: &[u8]) -> Vec<u8> {
            let res = keccak_256(input);
            let field_res = F::from_le_bytes_mod_order(&res);
            field_res.into_repr().to_bytes_le()
        }
    }

    use ark_bn254::Fr as Bn254;
    pub type Keccak256Bn254 = Keccak256<Bn254>;
}

#[allow(clippy::all)]
pub mod keccak {
    use super::hasher::Keccak256Bn254;

    /// The hash error types.
    #[derive(Debug)]
    pub enum Error {
        /// Returned if there is an error hashing
        HashError,
    }

    pub struct Keccak256;

    impl Keccak256 {
        pub fn hash(inputs: &[u8]) -> Result<[u8; 32], Error> {
            let res = Keccak256Bn254::hash(inputs, &[]);
            let out: [u8; 32] = res.try_into().map_err(|_| Error::HashError)?;
            Ok(out)
        }
    }
}
