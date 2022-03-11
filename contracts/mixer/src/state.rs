use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{CanonicalAddr, StdResult, Storage, Uint256};
use cw_storage_plus::{Item, Map};

use protocol_cosmwasm::error::ContractError;
use protocol_cosmwasm::mixer_verifier::MixerVerifier;
use protocol_cosmwasm::poseidon::Poseidon;
use protocol_cosmwasm::zeroes;

pub const ROOT_HISTORY_SIZE: u32 = 100;

// Mixer
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Mixer {
    pub initialized: bool,
    pub deposit_size: Uint256,
    pub merkle_tree: MerkleTree,
    pub cw20_address: Option<CanonicalAddr>,
}

pub const MIXER: Item<Mixer> = Item::new("mixer");

// "used nullifier" which stores if the "nullifier" is used or not.
pub const USED_NULLIFIERS: Map<Vec<u8>, bool> = Map::new("used_nullifers");

// "Poseidon hasher"
pub const POSEIDON: Item<Poseidon> = Item::new("poseidon");

// "MixerVerifier"
pub const MIXERVERIFIER: Item<MixerVerifier> = Item::new("mixer_verifier");

// MerkleTree
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MerkleTree {
    pub levels: u32,
    pub current_root_index: u32,
    pub next_index: u32,
}

// MerkleTree "filled_subtrees" Map
pub const FILLED_SUBTREES: Map<String, [u8; 32]> = Map::new("filled_subtrees");

pub fn save_subtree(store: &mut dyn Storage, k: u32, data: &[u8; 32]) -> StdResult<()> {
    FILLED_SUBTREES.save(store, k.to_string(), data)
}

pub fn read_subtree(store: &dyn Storage, k: u32) -> StdResult<[u8; 32]> {
    FILLED_SUBTREES.load(store, k.to_string())
}

// MerkleTree Roots Map
pub const MERKLE_ROOTS: Map<String, [u8; 32]> = Map::new("merkle_roots");

pub fn save_root(store: &mut dyn Storage, k: u32, data: &[u8; 32]) -> StdResult<()> {
    MERKLE_ROOTS.save(store, k.to_string(), data)
}

pub fn read_root(store: &dyn Storage, k: u32) -> StdResult<[u8; 32]> {
    MERKLE_ROOTS.load(store, k.to_string())
}

impl MerkleTree {
    fn hash_left_right(
        &self,
        hasher: Poseidon,
        left: [u8; 32],
        right: [u8; 32],
    ) -> Result<[u8; 32], ContractError> {
        let inputs = vec![left, right];
        hasher.hash(inputs).map_err(|_e| ContractError::HashError)
    }

    pub fn insert(
        &mut self,
        hasher: Poseidon,
        leaf: [u8; 32],
        store: &mut dyn Storage,
    ) -> Result<u32, ContractError> {
        let next_index = self.next_index;
        assert!(
            next_index != 2u32.pow(self.levels as u32),
            "Merkle tree is full"
        );

        let mut current_index = next_index;
        let mut current_level_hash = leaf;
        let mut left: [u8; 32];
        let mut right: [u8; 32];

        for i in 0..self.levels {
            if current_index % 2 == 0 {
                left = current_level_hash;
                right = zeroes::zeroes(i);
                save_subtree(store, i, &current_level_hash)?;
            } else {
                left = read_subtree(store, i).map_err(|_| ContractError::HashError)?;
                right = current_level_hash;
            }

            current_level_hash = self.hash_left_right(hasher.clone(), left, right)?;
            current_index /= 2;
        }

        let new_root_index = (self.current_root_index + 1) % ROOT_HISTORY_SIZE;
        self.current_root_index = new_root_index;
        save_root(store, new_root_index, &current_level_hash)?;
        self.next_index = next_index + 1;
        Ok(next_index)
    }

    pub fn is_known_root(&self, root: [u8; 32], store: &dyn Storage) -> bool {
        if root == [0u8; 32] {
            return false;
        }

        let mut i = self.current_root_index;
        for _ in 0..ROOT_HISTORY_SIZE {
            let r = read_root(store, i).unwrap_or([0u8; 32]);
            if r == root {
                return true;
            }

            if i == 0 {
                i = ROOT_HISTORY_SIZE - 1;
            } else {
                i -= 1;
            }
        }

        false
    }
}
