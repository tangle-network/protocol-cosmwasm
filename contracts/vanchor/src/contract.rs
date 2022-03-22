#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, from_binary, to_binary, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response,
    StdError, StdResult, Storage, Uint128, Uint256, WasmMsg,
};
use cw2::set_contract_version;

use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};
use protocol_cosmwasm::error::ContractError;
use protocol_cosmwasm::field_ops::{ArkworksIntoFieldBn254, IntoPrimeField};
use protocol_cosmwasm::keccak::Keccak256;
use protocol_cosmwasm::poseidon::Poseidon;
use protocol_cosmwasm::vanchor::{
    Cw20HookMsg, ExecuteMsg, InstantiateMsg, QueryMsg, UpdateConfigMsg,
};
use protocol_cosmwasm::vanchor_verifier::VAnchorVerifier;
use protocol_cosmwasm::zeroes::zeroes;

use crate::state::{
    read_curr_neighbor_root_index, save_curr_neighbor_root_index, save_edge, save_neighbor_roots,
    save_root, save_subtree, Edge, LinkableMerkleTree, MerkleTree, VAnchor, NULLIFIERS, POSEIDON,
    VANCHOR, VERIFIER_16_2, VERIFIER_2_2,
};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cosmwasm-vanchor";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// ChainType info
const COSMOS_CHAIN_TYPE: [u8; 2] = [4, 0]; // 0x0400

// History length for the "Curr_neighbor_root_index".
const HISTORY_LENGTH: u32 = 30;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    // Validation 1. Check if any funds are sent with this message.
    if !info.funds.is_empty() {
        return Err(ContractError::UnnecessaryFunds {});
    }

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Initialize the poseidon hasher
    POSEIDON.save(deps.storage, &Poseidon::new())?;

    // Initialize the vanchor verifiers
    let vk_bytes = include_bytes!(
        "../../../protocol-substrate-fixtures/vanchor/bn254/x5/2-2-2/verifying_key.bin"
    );
    VERIFIER_2_2.save(deps.storage, &VAnchorVerifier::new(vk_bytes))?;
    let vk_bytes = include_bytes!(
        "../../../protocol-substrate-fixtures/vanchor/bn254/x5/2-16-2/verifying_key.bin"
    );
    VERIFIER_16_2.save(deps.storage, &VAnchorVerifier::new(vk_bytes))?;

    // Initialize the merkle tree
    let merkle_tree: MerkleTree = MerkleTree {
        levels: msg.levels,
        current_root_index: 0,
        next_index: 0,
    };

    // Initialize the linkable merkle tree
    let linkable_merkle_tree = LinkableMerkleTree {
        max_edges: msg.max_edges,
        chain_id_list: Vec::new(),
    };
    // Get the "cw20_address"
    let cw20_address = deps.api.addr_canonicalize(&msg.cw20_address)?;

    // Initialize the VAnchor
    let anchor = VAnchor {
        chain_id: msg.chain_id,
        creator: deps.api.addr_canonicalize(info.sender.as_str())?,
        max_deposit_amt: msg.max_deposit_amt,
        min_withdraw_amt: msg.min_withdraw_amt,
        max_ext_amt: msg.max_ext_amt,
        max_fee: msg.max_fee,
        linkable_tree: linkable_merkle_tree,
        merkle_tree,
        cw20_address,
    };
    VANCHOR.save(deps.storage, &anchor)?;

    // Initialize the "FILLED_SUBTREES" with "zero" data.
    for i in 0..msg.levels {
        save_subtree(deps.storage, i as u32, &zeroes(i))?;
    }

    // Initialize the (merkletree) "ROOTS" with "zero" data.
    save_root(deps.storage, 0_u32, &zeroes(msg.levels))?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateConfig(msg) => update_vanchor_config(deps, info, msg),
        ExecuteMsg::Receive(msg) => transact(deps, info, msg),
        ExecuteMsg::AddEdge {
            src_chain_id,
            root,
            latest_leaf_index,
        } => add_edge(deps, info, src_chain_id, root, latest_leaf_index),
        ExecuteMsg::UpdateEdge {
            src_chain_id,
            root,
            latest_leaf_index,
        } => update_edge(deps, info, src_chain_id, root, latest_leaf_index),
    }
}

fn update_vanchor_config(
    deps: DepsMut,
    info: MessageInfo,
    msg: UpdateConfigMsg,
) -> Result<Response, ContractError> {
    // Validation 1. Check if any funds are sent with this message.
    if !info.funds.is_empty() {
        return Err(ContractError::UnnecessaryFunds {});
    }

    let mut vanchor = VANCHOR.load(deps.storage)?;
    // Validation 2. Check if the msg sender is "creator".
    if vanchor.creator != deps.api.addr_canonicalize(info.sender.as_str())? {
        return Err(ContractError::Unauthorized {});
    }

    // Update the vanchor config.
    if let Some(max_deposit_amt) = msg.max_deposit_amt {
        vanchor.max_deposit_amt = max_deposit_amt;
    }

    if let Some(min_withdraw_amt) = msg.min_withdraw_amt {
        vanchor.min_withdraw_amt = min_withdraw_amt;
    }

    if let Some(max_ext_amt) = msg.max_ext_amt {
        vanchor.max_ext_amt = max_ext_amt;
    }

    if let Some(max_fee) = msg.max_fee {
        vanchor.max_fee = max_fee;
    }

    VANCHOR.save(deps.storage, &vanchor)?;

    Ok(Response::new().add_attributes(vec![attr("method", "update_vanchor_config")]))
}

fn transact(
    deps: DepsMut,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    // Only Cw20 token contract can execute this message.
    let vanchor: VAnchor = VANCHOR.load(deps.storage)?;
    if vanchor.cw20_address != deps.api.addr_canonicalize(info.sender.as_str())? {
        return Err(ContractError::Unauthorized {});
    }

    // let transactor = cw20_msg.sender;
    let cw20_token_amt = cw20_msg.amount;
    let cw20_address = info.sender.to_string();

    match from_binary(&cw20_msg.msg) {
        Ok(Cw20HookMsg::Transact {
            proof_data,
            ext_data,
        }) => {
            let ext_data_fee: u128 = ext_data.fee.parse().expect("Invalid ext_fee");
            let ext_amt: i128 = ext_data.ext_amount.parse().expect("Invalid ext_amount");

            // Validation 1. Double check the number of roots.
            if vanchor.linkable_tree.max_edges != proof_data.roots.len() as u32 {
                return Err(ContractError::Std(StdError::GenericErr {
                    msg: "Max edges not matched".to_string(),
                }));
            }

            // Validation 2. Check if the root is known to merkle tree
            if !vanchor
                .merkle_tree
                .is_known_root(proof_data.roots[0], deps.storage)
            {
                return Err(ContractError::Std(StdError::GenericErr {
                    msg: "Root is not known".to_string(),
                }));
            }

            // Validation 3. Check if the roots are valid in linkable tree.
            let linkable_tree = vanchor.linkable_tree;
            if !linkable_tree.is_valid_neighbor_roots(&proof_data.roots[1..], deps.storage) {
                return Err(ContractError::Std(StdError::GenericErr {
                    msg: "Neighbor roots are not valid".to_string(),
                }));
            }

            // Check nullifier and add or return `InvalidNullifier`
            for nullifier in &proof_data.input_nullifiers {
                if is_known_nullifier(deps.storage, *nullifier) {
                    return Err(ContractError::Std(StdError::GenericErr {
                        msg: "Nullifier is known".to_string(),
                    }));
                }
            }

            let element_encoder = |v: &[u8]| {
                let mut output = [0u8; 32];
                output.iter_mut().zip(v).for_each(|(b1, b2)| *b1 = *b2);
                output
            };

            // Compute hash of abi encoded ext_data, reduced into field from config
            // Ensure that the passed external data hash matches the computed one
            let mut ext_data_args = Vec::new();
            let recipient_bytes = element_encoder(
                &hex::decode(&ext_data.recipient).map_err(|_| ContractError::DecodeError)?,
            );
            let relayer_bytes = element_encoder(
                &hex::decode(&ext_data.relayer).map_err(|_| ContractError::DecodeError)?,
            );
            let fee_bytes = element_encoder(&ext_data_fee.to_le_bytes());
            let ext_amt_bytes = element_encoder(&ext_amt.to_le_bytes());
            ext_data_args.extend_from_slice(&recipient_bytes);
            ext_data_args.extend_from_slice(&relayer_bytes);
            ext_data_args.extend_from_slice(&ext_amt_bytes);
            ext_data_args.extend_from_slice(&fee_bytes);
            ext_data_args.extend_from_slice(&ext_data.encrypted_output1);
            ext_data_args.extend_from_slice(&ext_data.encrypted_output2);

            let computed_ext_data_hash =
                Keccak256::hash(&ext_data_args).map_err(|_| ContractError::HashError)?;
            if computed_ext_data_hash != proof_data.ext_data_hash {
                return Err(ContractError::Std(StdError::GenericErr {
                    msg: "Invalid ext data".to_string(),
                }));
            }

            let abs_ext_amt = ext_amt.unsigned_abs();
            // Making sure that public amount and fee are correct
            if Uint256::from(ext_data_fee) > vanchor.max_fee {
                return Err(ContractError::Std(StdError::GenericErr {
                    msg: "Invalid fee amount".to_string(),
                }));
            }

            if Uint256::from(abs_ext_amt) > vanchor.max_ext_amt {
                return Err(ContractError::Std(StdError::GenericErr {
                    msg: "Invalid ext amount".to_string(),
                }));
            }

            // Public amounnt can also be negative, in which
            // case it would wrap around the field, so we should check if FIELD_SIZE -
            // public_amount == proof_data.public_amount, in case of a negative ext_amount
            let calc_public_amt = ext_amt - ext_data_fee as i128;
            let calc_public_amt_bytes =
                element_encoder(&ArkworksIntoFieldBn254::into_field(calc_public_amt));
            if calc_public_amt_bytes != proof_data.public_amount {
                return Err(ContractError::Std(StdError::GenericErr {
                    msg: "Invalid public amount".to_string(),
                }));
            }

            // Construct public inputs
            let chain_id_type_bytes = element_encoder(
                &compute_chain_id_type(vanchor.chain_id, &COSMOS_CHAIN_TYPE).to_le_bytes(),
            );

            let mut bytes = Vec::new();
            bytes.extend_from_slice(&proof_data.public_amount);
            bytes.extend_from_slice(&proof_data.ext_data_hash);
            for null in &proof_data.input_nullifiers {
                bytes.extend_from_slice(null);
            }
            for comm in &proof_data.output_commitments {
                bytes.extend_from_slice(comm);
            }
            bytes.extend_from_slice(&element_encoder(&chain_id_type_bytes));
            for root in &proof_data.roots {
                bytes.extend_from_slice(root);
            }

            let verifier_2_2 = VERIFIER_2_2.load(deps.storage)?;
            let verifier_16_2 = VERIFIER_16_2.load(deps.storage)?;

            let result = match (
                proof_data.input_nullifiers.len(),
                proof_data.output_commitments.len(),
            ) {
                (2, 2) => verify(verifier_2_2, bytes, proof_data.proof)?,
                (16, 2) => verify(verifier_16_2, bytes, proof_data.proof)?,
                _ => false,
            };

            if !result {
                return Err(ContractError::Std(StdError::GenericErr {
                    msg: "Invalid transaction proof".to_string(),
                }));
            }

            // Flag nullifiers as used
            for nullifier in &proof_data.input_nullifiers {
                NULLIFIERS.save(deps.storage, nullifier.to_vec(), &true)?;
            }

            // Deposit or Withdraw
            let mut msgs: Vec<CosmosMsg> = vec![];

            let is_deposit = ext_amt.is_positive();
            if is_deposit {
                if Uint256::from(abs_ext_amt) > vanchor.max_deposit_amt {
                    return Err(ContractError::Std(StdError::GenericErr {
                        msg: "Invalid deposit amount".to_string(),
                    }));
                };
                if abs_ext_amt != cw20_token_amt.u128() {
                    return Err(ContractError::Std(StdError::GenericErr {
                        msg: "Did not send enough tokens".to_string(),
                    }));
                };
                // No need to call "transfer from transactor to this contract"
                // since this message is the result of sending.
            } else {
                if Uint256::from(abs_ext_amt) < vanchor.min_withdraw_amt {
                    return Err(ContractError::Std(StdError::GenericErr {
                        msg: "Invalid withdraw amount".to_string(),
                    }));
                }
                if cw20_token_amt != Uint128::zero() {
                    return Err(ContractError::Std(StdError::GenericErr {
                        msg: "Sent unnecessary funds".to_string(),
                    }));
                }
                msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: cw20_address.clone(),
                    funds: [].to_vec(),
                    msg: to_binary(&Cw20ExecuteMsg::Transfer {
                        recipient: ext_data.recipient.clone(),
                        amount: Uint128::try_from(abs_ext_amt).unwrap(),
                    })?,
                }));
            }

            // If fee exists, handle it
            let fee_exists = ext_data_fee != 0;

            if fee_exists {
                msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: cw20_address,
                    funds: [].to_vec(),
                    msg: to_binary(&Cw20ExecuteMsg::Transfer {
                        recipient: ext_data.relayer.clone(),
                        amount: Uint128::try_from(ext_data_fee).unwrap(),
                    })?,
                }));
            }

            // Insert output commitments into the tree
            let mut merkle_tree = vanchor.merkle_tree;
            for comm in &proof_data.output_commitments {
                let poseidon: Poseidon = POSEIDON.load(deps.storage)?;
                merkle_tree.insert(poseidon, *comm, deps.storage)?;
            }

            VANCHOR.save(
                deps.storage,
                &VAnchor {
                    creator: vanchor.creator,
                    chain_id: vanchor.chain_id,
                    merkle_tree,
                    linkable_tree,
                    cw20_address: vanchor.cw20_address,
                    max_deposit_amt: vanchor.max_deposit_amt,
                    min_withdraw_amt: vanchor.min_withdraw_amt,
                    max_fee: vanchor.max_fee,
                    max_ext_amt: vanchor.max_ext_amt,
                },
            )?;

            Ok(Response::new().add_messages(msgs).add_attributes(vec![
                attr("method", "transact"),
                attr("deposit", is_deposit.to_string()),
                attr("withdraw", (!is_deposit).to_string()),
                attr("ext_amt", ext_amt.to_string()),
            ]))
        }
        Err(_) => Err(ContractError::Std(StdError::generic_err(
            "invalid cw20 hook msg",
        ))),
    }
}

fn add_edge(
    deps: DepsMut,
    info: MessageInfo,
    src_chain_id: u64,
    root: [u8; 32],
    latest_leaf_index: u32,
) -> Result<Response, ContractError> {
    // Validation 1. Check if any funds are sent with this message.
    if !info.funds.is_empty() {
        return Err(ContractError::UnnecessaryFunds {});
    }

    let vanchor = VANCHOR.load(deps.storage)?;
    let linkable_tree = vanchor.linkable_tree;
    if linkable_tree.has_edge(src_chain_id, deps.storage) {
        return Err(ContractError::Std(StdError::GenericErr {
            msg: "Edge already exists".to_string(),
        }));
    }

    let curr_length = linkable_tree.get_latest_neighbor_edges(deps.storage).len();
    if curr_length > linkable_tree.max_edges as usize {
        return Err(ContractError::Std(StdError::GenericErr {
            msg: "Too many edges".to_string(),
        }));
    }

    // craft edge
    let edge: Edge = Edge {
        chain_id: src_chain_id,
        root,
        latest_leaf_index,
    };

    // update historical neighbor list for this edge's root
    let curr_neighbor_root_idx = read_curr_neighbor_root_index(deps.storage, src_chain_id)?;
    save_curr_neighbor_root_index(
        deps.storage,
        src_chain_id,
        (curr_neighbor_root_idx + 1) % HISTORY_LENGTH,
    )?;

    save_neighbor_roots(deps.storage, (src_chain_id, curr_neighbor_root_idx), root)?;

    // Append new edge to the end of the edge list for the given tree
    save_edge(deps.storage, src_chain_id, edge)?;

    Ok(Response::new().add_attributes(vec![attr("method", "add_edge")]))
}

fn update_edge(
    deps: DepsMut,
    info: MessageInfo,
    src_chain_id: u64,
    root: [u8; 32],
    latest_leaf_index: u32,
) -> Result<Response, ContractError> {
    // Validation 1. Check if any funds are sent with this message.
    if !info.funds.is_empty() {
        return Err(ContractError::UnnecessaryFunds {});
    }

    let vanchor = VANCHOR.load(deps.storage)?;
    let linkable_tree = vanchor.linkable_tree;
    if !linkable_tree.has_edge(src_chain_id, deps.storage) {
        return Err(ContractError::Std(StdError::GenericErr {
            msg: "Edge does not exist".to_string(),
        }));
    }

    let edge: Edge = Edge {
        chain_id: src_chain_id,
        root,
        latest_leaf_index,
    };
    let neighbor_root_idx =
        (read_curr_neighbor_root_index(deps.storage, src_chain_id)? + 1) % HISTORY_LENGTH;
    save_curr_neighbor_root_index(deps.storage, src_chain_id, neighbor_root_idx)?;
    save_neighbor_roots(deps.storage, (src_chain_id, neighbor_root_idx), root)?;

    save_edge(deps.storage, src_chain_id, edge)?;

    Ok(Response::new().add_attributes(vec![attr("method", "udpate_edge")]))
}

// Check if the "nullifier" is already used or not.
fn is_known_nullifier(store: &dyn Storage, nullifier: [u8; 32]) -> bool {
    NULLIFIERS.has(store, nullifier.to_vec())
}

// Truncate and pad 256 bit slice
// NOTE: remove `pub`
pub fn truncate_and_pad(t: &[u8]) -> Vec<u8> {
    let mut truncated_bytes = t[..20].to_vec();
    truncated_bytes.extend_from_slice(&[0u8; 12]);
    truncated_bytes
}

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

// Using "anchor_verifier", verifies if the "proof" really came from "public_input".
fn verify(
    verifier: VAnchorVerifier,
    public_input: Vec<u8>,
    proof_bytes: Vec<u8>,
) -> Result<bool, ContractError> {
    verifier
        .verify(public_input, proof_bytes)
        .map_err(|_| ContractError::VerifyError)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        // TODO
    }
}
