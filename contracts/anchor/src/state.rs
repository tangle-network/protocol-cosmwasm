use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{ Storage, StdResult, Uint256};
use cw_storage_plus::{Item, Map};

use protocol_cosmwasm::error::ContractError;
use protocol_cosmwasm::poseidon::Poseidon;
use protocol_cosmwasm::anchor_verifier::AnchorVerifier;
use protocol_cosmwasm::zeroes;

const ROOT_HISTORY_SIZE: u32 = 100;

pub type ChainId = u64;


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Default, Copy)]
pub struct Edge {
    pub chain_id: ChainId,
    pub root: [u8; 32],
    pub latest_leaf_index: u32,
}


// LinkableMerkleTree
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct LinkableMerkleTree {
    pub max_edges: u32,
    pub chain_id_list: Vec<ChainId>,
    // pub edges: Map<ChainId, Edge>,
    // pub curr_neighbor_root_index: Mapping<ChainId, u32>,
    // pub neighbor_roots: Mapping<(ChainId, u32), [u8; 32]>,
}

// LinkableMerkleTree "edges" map
pub const EDGES: Map<String, Edge> = Map::new("edges");

pub fn read_edge(store: &dyn Storage, k: ChainId) -> StdResult<Edge> {
    EDGES.load(store, k.to_string())
}

pub fn save_edge(store: &mut dyn Storage, k: ChainId, data: Edge) -> StdResult<()> {
    EDGES.save(store, k.to_string(), &data)
}

pub fn has_edge(store: &dyn Storage, k: ChainId) -> bool {
    EDGES.has(store, k.to_string())
}

// LinkableMerkleTree "curr_neighbor_root_index" map
pub const CURR_NEIGHBOR_ROOT_INDEX: Map<String, u32> = Map::new("curr_neighbor_root_index");

pub fn read_curr_neighbor_root_index(store: &dyn Storage, k: ChainId) -> StdResult<u32> {
    CURR_NEIGHBOR_ROOT_INDEX.load(store, k.to_string())
}

pub fn save_curr_neighbor_root_index(store: &mut dyn Storage, k: ChainId, data: u32) -> StdResult<()> {
    CURR_NEIGHBOR_ROOT_INDEX.save(store, k.to_string(), &data)
}

// LinkableMerkleTree "neighbor_roots" map
pub const NEIGHBOR_ROOTS: Map<(String, String), [u8; 32]> = Map::new("neighbor_roots");

pub fn read_neighbor_roots(store: &dyn Storage, k: (ChainId, u32)) -> StdResult<[u8; 32]> {
    let (id, num) = k;
    NEIGHBOR_ROOTS.load(store, (id.to_string(), num.to_string()))
}

pub fn save_neighbor_roots(store: &mut dyn Storage, k: (ChainId, u32), data: [u8; 32]) -> StdResult<()> {
    let (id, num) = k;
    NEIGHBOR_ROOTS.save(store, (id.to_string(), num.to_string()), &data)
}

impl LinkableMerkleTree {
    fn has_edge(&self, chain_id: ChainId, store: &dyn Storage) -> bool {
        // self.edges.get(&chain_id).is_some()
        has_edge(store, chain_id)
    }

    pub fn update_edge(&mut self, edge: Edge, store: &mut dyn Storage) -> Result<(), ContractError> {
        if has_edge(store, edge.chain_id) {
        // if self.has_edge(edge.chain_id) {
            let leaf_index = read_edge(store, edge.chain_id).unwrap_or_default().latest_leaf_index + 65_536;
            assert!(
                edge.latest_leaf_index < leaf_index,
                "latest leaf index should be greater than the previous one"
            );
            // self.edges.insert(edge.chain_id, &edge);
            save_edge(store, edge.chain_id, edge)?;

            // let curr_neighbor_root_index = self.curr_neighbor_root_index.get(&edge.chain_id).unwrap_or_default();
            let curr_neighbor_root_index = read_curr_neighbor_root_index(store, edge.chain_id).unwrap_or_default();
            let neighbor_root_index = curr_neighbor_root_index + 1 % ROOT_HISTORY_SIZE;

            // self.curr_neighbor_root_index.insert(edge.chain_id, &neighbor_root_index);
            save_curr_neighbor_root_index(store, edge.chain_id, neighbor_root_index)?;

            // self.neighbor_roots.insert((edge.chain_id, neighbor_root_index), &edge.root);
            save_neighbor_roots(store, (edge.chain_id, neighbor_root_index), edge.root)?;
        } else {
            let edge_count = self.chain_id_list.len() as u32;
            assert!(self.max_edges >= edge_count as u32 + 1, "Edge list is full");
            // self.edges.insert(edge.chain_id, &edge);
            save_edge(store, edge.chain_id, edge)?;
            // self.neighbor_roots.insert((edge.chain_id, 1), &edge.root);
            save_neighbor_roots(store, (edge.chain_id, 1), edge.root)?;
            // self.curr_neighbor_root_index.insert(edge.chain_id, &1);
            save_curr_neighbor_root_index(store, edge.chain_id, 1)?;
            self.chain_id_list.push(edge.chain_id);
        }

        Ok(())
    }

    pub fn get_latest_neighbor_root(&self, chain_id: ChainId, store: &dyn Storage) -> Result<[u8; 32], ContractError> {
        // let neighbor_root_index = self.curr_neighbor_root_index.get(&chain_id).ok_or(ContractError::ItemNotFound)?;
        let neighbor_root_index = read_curr_neighbor_root_index(store, chain_id).map_err(|_| ContractError::ItemNotFound)?;
        // let latest_neighbor_root = self.neighbor_roots.get(&(chain_id, neighbor_root_index)).ok_or(anchor::Error::ItemNotFound)?;
        let latest_neighbor_root = read_neighbor_roots(store, (chain_id, neighbor_root_index)).map_err(|_| ContractError::ItemNotFound)?;
        Ok(latest_neighbor_root)
    }

    pub fn get_latest_neighbor_edges(&self, store: &dyn Storage) -> Vec<Edge> {
        // self.chain_id_list.iter().map(|c_id| self.edges.get(c_id).unwrap_or_default()).collect()
        self.chain_id_list.iter().map(|c_id| read_edge(store, *c_id).unwrap_or_default()).collect()
    }

    pub fn get_neighbor_roots(&self, store: &dyn Storage) -> Vec<[u8; 32]> {
        // self.chain_id_list.iter().map(|c_id| self.edges.get(c_id).unwrap_or_default().root).collect()
        self.chain_id_list.iter().map(|c_id| read_edge(store, *c_id).unwrap_or_default().root).collect()
    }

    pub fn is_known_neighbor_root(&self, chain_id: ChainId, root: [u8; 32], store: &dyn Storage) -> bool {
        if root == [0u8; 32] {
            return false;
        }

        // let mut i = self.curr_neighbor_root_index.get(&chain_id).unwrap_or_default();
        let mut i = read_curr_neighbor_root_index(store, chain_id).unwrap_or_default();
        for _ in 0..ROOT_HISTORY_SIZE {
            if let Ok(r) = read_neighbor_roots(store, (chain_id, i)) {
            // if let Some(r) = self.neighbor_roots.get(&(chain_id, i)) {
                if r == root {
                    return true;
                }
    
                if i == 0 {
                    i = ROOT_HISTORY_SIZE - 1;
                } else {
                    i -= 1;
                }
            }
        }

        false
    }

    pub fn is_valid_neighbor_roots(&self, roots: &[[u8; 32]], store: &dyn Storage) -> bool {
        assert!(roots.len() == self.max_edges as usize - 1, "Incorrect roots length");
        for (i, edge) in self.chain_id_list.iter().map(|c_id| read_edge(store, *c_id).unwrap_or_default()).enumerate() {
            if !self.is_known_neighbor_root(edge.chain_id, roots[i], store) {
                return false;
            }
        }
        return true;
    }
}

// Anchor struct
// TODO: Anchor should have an ERC20 attached
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Anchor {
    pub chain_id: u64,
    pub deposit_size: Uint256,
    pub merkle_tree: MerkleTree,
    pub linkable_tree: LinkableMerkleTree,
    // used_nullifiers: Mapping<[u8; 32], bool>,
    // poseidon: PoseidonRef,
    // verifier: AnchorVerifierRef,
}

pub const ANCHOR: Item<Anchor> = Item::new("anchor");

// Struct to save the use of "nullifiers"
pub const NULLIFIERS: Map<Vec<u8>, bool> = Map::new("used_nullifers");

// "Poseidon hasher"
pub const POSEIDON: Item<Poseidon> = Item::new("poseidon");

// "AnchorVerifier"
pub const ANCHORVERIFIER: Item<AnchorVerifier> = Item::new("anchor_verifier");


// MerkleTree
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MerkleTree {
    pub levels: u32,
    pub current_root_index: u32,
    pub next_index: u32,
    // pub filled_subtrees: HashMap<u32, [u8; 32]>,
    // pub roots: HashMap<u32, [u8; 32]>
}

// MerkleTree "filled_subtrees" Map
pub const FILLEDSUBTREES: Map<String, [u8; 32]> = Map::new("filled_subtrees");

pub fn save_subtree(store: &mut dyn Storage, k: u32, data: &[u8; 32]) -> StdResult<()> {
    FILLEDSUBTREES.save(store, k.to_string(), data)
}

pub fn read_subtree(store: &dyn Storage, k: u32) -> StdResult<[u8; 32]> {
    FILLEDSUBTREES.load(store, k.to_string())
}

// MerkleTree Roots Map
pub const MERKLEROOTS: Map<String, [u8; 32]> = Map::new("merkle_roots");

pub fn save_root(store: &mut dyn Storage, k: u32, data: &[u8; 32]) -> StdResult<()> {
    MERKLEROOTS.save(store, k.to_string(), data)
}

pub fn read_root(store: &dyn Storage, k: u32) -> StdResult<[u8; 32]> {
    MERKLEROOTS.load(store, k.to_string())
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
