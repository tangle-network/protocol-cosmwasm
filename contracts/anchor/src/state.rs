use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Storage, StdResult};
use cw_storage_plus::{Item, Map};

use protocol_cosmwasm::error::ContractError;

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
    // pub edges: Mapping<ChainId, Edge>,
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
        assert!(roots.len() == self.max_edges as usize, "Incorrect roots length");
        for (i, edge) in self.chain_id_list.iter().map(|c_id| read_edge(store, *c_id).unwrap_or_default()).enumerate() {
            if !self.is_known_neighbor_root(edge.chain_id, roots[i], store) {
                return false;
            }
        }
        return true;
    }
}


