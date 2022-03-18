#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, from_binary, to_binary, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response,
    StdError, StdResult, Storage, Uint128, Uint256, WasmMsg,
};
use cw2::set_contract_version;

use sp_core::hashing::keccak_256;

use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};
use protocol_cosmwasm::error::ContractError;
use protocol_cosmwasm::poseidon::Poseidon;
use protocol_cosmwasm::vanchor::{
    Cw20HookMsg, ExecuteMsg, ExtData, InstantiateMsg, ProofData, QueryMsg, UpdateConfigMsg,
};
use protocol_cosmwasm::vanchor_verifier::VAnchorVerifier;
use protocol_cosmwasm::zeroes::zeroes;

use crate::state::{
    read_curr_neighbor_root_index, save_curr_neighbor_root_index, save_edge, save_neighbor_roots,
    save_root, save_subtree, Edge, LinkableMerkleTree, MerkleTree, VAnchor, NULLIFIERS, POSEIDON,
    VANCHOR, VANCHORVERIFIER,
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

    // Initialize the VAnchor_verifier
    VANCHORVERIFIER.save(deps.storage, &VAnchorVerifier::new())?;

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
            is_deposit,
        }) => {
            // Validation 1. Double check the number of roots.
            assert!(
                vanchor.linkable_tree.max_edges == proof_data.roots.len() as u32,
                "Max edges not matched"
            );

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
            let fee_bytes = element_encoder(&ext_data.fee.to_le_bytes());
            let ext_amt_bytes = element_encoder(&ext_data.ext_amount.to_le_bytes());
            ext_data_args.extend_from_slice(&recipient_bytes);
            ext_data_args.extend_from_slice(&relayer_bytes);
            ext_data_args.extend_from_slice(&ext_amt_bytes);
            ext_data_args.extend_from_slice(&fee_bytes);
            ext_data_args.extend_from_slice(&ext_data.encrypted_output1);
            ext_data_args.extend_from_slice(&ext_data.encrypted_output2);

            let computed_ext_data_hash = keccak_256(&ext_data_args);
            assert!(
                computed_ext_data_hash == proof_data.ext_data_hash,
                "Invalid ext data"
            );

            // Making sure that public amount and fee are correct
            assert!(ext_data.fee < vanchor.max_fee, "Invalid fee amount");
            assert!(
                ext_data.ext_amount < vanchor.max_ext_amt,
                "Invalid ext amount"
            );

            // Public amounnt can also be negative, in which
            // case it would wrap around the field, so we should check if FIELD_SIZE -
            // public_amount == proof_data.public_amount, in case of a negative ext_amount
            let calc_public_amt = ext_data.ext_amount - ext_data.fee;
            let calc_public_amt_bytes = calc_public_amt.to_le_bytes();
            assert!(
                calc_public_amt_bytes == proof_data.public_amount.to_le_bytes(),
                "Invalid public amount"
            );

            // Construct public inputs
            let chain_id_type_bytes = element_encoder(
                &compute_chain_id_type(vanchor.chain_id, &COSMOS_CHAIN_TYPE).to_le_bytes(),
            );

            let mut bytes = Vec::new();
            bytes.extend_from_slice(&proof_data.public_amount.to_le_bytes());
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

            let verifier = VANCHORVERIFIER.load(deps.storage)?;
            let result = match (
                proof_data.input_nullifiers.len(),
                proof_data.output_commitments.len(),
            ) {
                (2, 2) => verify(verifier, bytes, proof_data.proof)?,
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
            let ext_amt = ext_data.ext_amount;
            if is_deposit {
                assert!(ext_amt <= vanchor.max_deposit_amt, "Invalid deposit amount");
                assert!(
                    ext_amt == Uint256::from(cw20_token_amt.u128()),
                    "Did not send enough tokens"
                );
                // No need to call "transfer from transactor to this contract"
                // since this message is the result of sending.
            } else {
                assert!(
                    ext_amt >= vanchor.min_withdraw_amt,
                    "Invalid withdraw amount"
                );
                assert!(cw20_token_amt == Uint128::zero(), "Sent unnecesary funds");

                msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: cw20_address.clone(),
                    funds: [].to_vec(),
                    msg: to_binary(&Cw20ExecuteMsg::Transfer {
                        recipient: ext_data.recipient.clone(),
                        amount: Uint128::try_from(ext_amt).unwrap(),
                    })?,
                }));
            }

            // If fee exists, handle it
            let fee_exists = !ext_data.fee.is_zero();

            if fee_exists {
                msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: cw20_address.clone(),
                    funds: [].to_vec(),
                    msg: to_binary(&Cw20ExecuteMsg::Transfer {
                        recipient: ext_data.relayer.clone(),
                        amount: Uint128::try_from(ext_data.fee).unwrap(),
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
                    merkle_tree: merkle_tree,
                    linkable_tree,
                    cw20_address: vanchor.cw20_address,
                    max_deposit_amt: vanchor.max_deposit_amt,
                    min_withdraw_amt: vanchor.min_withdraw_amt,
                    max_fee: vanchor.max_fee,
                    max_ext_amt: vanchor.max_ext_amt,
                },
            )?;

            return Ok(Response::new().add_messages(msgs).add_attributes(vec![
                attr("method", "transact"),
                attr("deposit", is_deposit.to_string()),
                attr("withdraw", (!is_deposit).to_string()),
                attr("ext_amt", ext_amt.to_string()),
            ]));
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
fn truncate_and_pad(t: &[u8]) -> Vec<u8> {
    let mut truncated_bytes = t[..20].to_vec();
    truncated_bytes.extend_from_slice(&[0u8; 12]);
    truncated_bytes
}

// Computes the combination bytes of "chain_type" and "chain_id".
// Combination rule: 8 bytes array(00 * 2 bytes + [chain_type] 2 bytes + [chain_id] 4 bytes)
// Example:
//  chain_type - 0x0401, chain_id - 0x00000001 (big endian)
//  Result - [00, 00, 04, 01, 00, 00, 00, 01]
fn compute_chain_id_type(chain_id: u64, chain_type: &[u8]) -> u64 {
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
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        // TODO
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_ff::BigInteger;
    use ark_ff::PrimeField;
    use arkworks_setups::Curve;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{attr, Uint128};
    use sp_core::hashing::keccak_256;

    fn element_encoder(v: &[u8]) -> [u8; 32] {
        let mut output = [0u8; 32];
        output.iter_mut().zip(v).for_each(|(b1, b2)| *b1 = *b2);
        output
    }

    #[test]
    fn proper_initialization() {
        let cw20_address = "terra1fex9f78reuwhfsnc8sun6mz8rl9zwqh03fhwf3".to_string();
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg {
            chain_id: 1,
            max_edges: 0,
            levels: 0,
            max_deposit_amt: Uint256::zero(),
            min_withdraw_amt: Uint256::zero(),
            max_ext_amt: Uint256::zero(),
            max_fee: Uint256::zero(),
            cw20_address: cw20_address,
        };
        let info = mock_info("creator", &[]);

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn test_vanchor_update_config() {
        let cw20_address = "terra1fex9f78reuwhfsnc8sun6mz8rl9zwqh03fhwf3".to_string();
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg {
            chain_id: 1,
            max_edges: 0,
            levels: 0,
            max_deposit_amt: Uint256::zero(),
            min_withdraw_amt: Uint256::zero(),
            max_ext_amt: Uint256::zero(),
            max_fee: Uint256::zero(),
            cw20_address: cw20_address,
        };
        let info = mock_info("creator", &[]);

        // we can just call .unwrap() to assert this was a success
        let _ = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Fail to update the config with "unauthorized" error.
        let update_config_msg = UpdateConfigMsg {
            max_deposit_amt: Some(Uint256::from(1u128)),
            min_withdraw_amt: Some(Uint256::from(1u128)),
            max_ext_amt: Some(Uint256::from(1u128)),
            max_fee: Some(Uint256::from(1u128)),
        };
        let info = mock_info("intruder", &[]);
        assert!(
            execute(
                deps.as_mut(),
                mock_env(),
                info,
                ExecuteMsg::UpdateConfig(update_config_msg)
            )
            .is_err(),
            "Should fail with unauthorized",
        );

        // We can just call .unwrap() to assert "execute" was success
        let update_config_msg = UpdateConfigMsg {
            max_deposit_amt: Some(Uint256::from(1u128)),
            min_withdraw_amt: Some(Uint256::from(1u128)),
            max_ext_amt: Some(Uint256::from(1u128)),
            max_fee: Some(Uint256::from(1u128)),
        };
        let info = mock_info("creator", &[]);
        let _ = execute(
            deps.as_mut(),
            mock_env(),
            info,
            ExecuteMsg::UpdateConfig(update_config_msg),
        )
        .unwrap();
    }

    #[test]
    fn test_vanchor_transact_deposit_cw20() {
        // Instantiate the "vanchor" contract.
        let cw20_address = "terra1fex9f78reuwhfsnc8sun6mz8rl9zwqh03fhwf3".to_string();
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg {
            chain_id: 1,
            max_edges: 2,
            levels: 30,
            max_deposit_amt: Uint256::from(40u128),
            min_withdraw_amt: Uint256::from(0u128),
            max_ext_amt: Uint256::from(20u128),
            max_fee: Uint256::from(10u128),
            cw20_address: cw20_address.clone(),
        };
        let info = mock_info("creator", &[]);

        // we can just call .unwrap() to assert this was a success
        let _ = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Initialize the vanchor
        let (pk_bytes, _) = crate::test_util::setup_environment(Curve::Bn254);
        let transactor = [1u8; 32];
        let recipient_bytes = [2u8; 32];
        let relayer_bytes = [0u8; 32];
        let ext_amount = 10;
        let fee = 0;

        let public_amount = 10;

        let chain_type = [4, 0];
        let chain_id = compute_chain_id_type(1, &chain_type);
        let in_chain_ids = [chain_id; 2];
        let in_amounts = [0, 0];
        let in_indices = [0, 1];
        let out_chain_ids = [chain_id; 2];
        let out_amounts = [10, 0];

        let in_utxos = crate::test_util::setup_utxos(in_chain_ids, in_amounts, Some(in_indices));
        // We are adding indices to out utxos, since they will be used as an input utxos in next transaction
        let out_utxos = crate::test_util::setup_utxos(out_chain_ids, out_amounts, Some(in_indices));

        let output1 = out_utxos[0].commitment.into_repr().to_bytes_le();
        let output2 = out_utxos[1].commitment.into_repr().to_bytes_le();

        let ext_data = ExtData {
            recipient: hex::encode(recipient_bytes),
            relayer: hex::encode(relayer_bytes),
            ext_amount: Uint256::from(ext_amount as u128),
            fee: Uint256::from(fee as u128),
            encrypted_output1: element_encoder(&output1),
            encrypted_output2: element_encoder(&output2),
        };

        let mut ext_data_args = Vec::new();
        let recipient_bytes = element_encoder(&hex::decode(&ext_data.recipient).unwrap());
        let relayer_bytes = element_encoder(&hex::decode(&ext_data.relayer).unwrap());
        let fee_bytes = element_encoder(&ext_data.fee.to_le_bytes());
        let ext_amt_bytes = element_encoder(&ext_data.ext_amount.to_le_bytes());
        ext_data_args.extend_from_slice(&recipient_bytes);
        ext_data_args.extend_from_slice(&relayer_bytes);
        ext_data_args.extend_from_slice(&ext_amt_bytes);
        ext_data_args.extend_from_slice(&fee_bytes);
        ext_data_args.extend_from_slice(&ext_data.encrypted_output1);
        ext_data_args.extend_from_slice(&ext_data.encrypted_output2);

        let ext_data_hash = keccak_256(&ext_data_args);
        let custom_roots = Some([[0u8; 32]; 2].map(|x| x.to_vec()));
        let (proof, public_inputs) = crate::test_util::setup_zk_circuit(
            public_amount,
            chain_id,
            ext_data_hash.to_vec(),
            in_utxos,
            out_utxos,
            custom_roots,
            pk_bytes,
        );

        // Deconstructing public inputs
        let (_chain_id, public_amount, root_set, nullifiers, commitments, ext_data_hash) =
            crate::test_util::deconstruct_public_inputs_el(&public_inputs);

        // Constructing proof data
        let root_set = root_set.into_iter().map(|v| v.0).collect();
        let nullifiers = nullifiers.into_iter().map(|v| v.0).collect();
        let commitments = commitments.into_iter().map(|v| v.0).collect();
        let proof_data = ProofData::new(
            proof,
            Uint256::from(10u128),
            root_set,
            nullifiers,
            commitments,
            ext_data_hash.0,
        );

        // Should "transact" with success.
        let info = mock_info(cw20_address.as_str(), &[]);
        let deposit_cw20_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: cw20_address.clone(),
            amount: Uint128::from(10u128),
            msg: to_binary(&Cw20HookMsg::Transact {
                proof_data: proof_data,
                ext_data: ext_data,
                is_deposit: true,
            })
            .unwrap(),
        });

        let response = execute(deps.as_mut(), mock_env(), info, deposit_cw20_msg).unwrap();
        assert_eq!(
            response.attributes,
            vec![
                attr("method", "transact"),
                attr("deposit", "true"),
                attr("withdraw", "false"),
                attr("ext_amt", "10"),
            ]
        );
    }
}
