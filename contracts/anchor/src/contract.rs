#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response, StdError,
    StdResult, Storage, Uint128, Uint256, from_binary, to_binary, WasmMsg,
};
use cw2::set_contract_version;

use cw20::{Cw20ReceiveMsg, Cw20ExecuteMsg};
use protocol_cosmwasm::anchor::{DepositMsg, ExecuteMsg, InstantiateMsg, QueryMsg, WithdrawMsg, Cw20HookMsg};
use protocol_cosmwasm::anchor_verifier::AnchorVerifier;
use protocol_cosmwasm::error::ContractError;
use protocol_cosmwasm::poseidon::Poseidon;
use protocol_cosmwasm::zeroes::zeroes;

use crate::state::{
    save_root, save_subtree, Anchor, LinkableMerkleTree, MerkleTree, ANCHOR, ANCHORVERIFIER,
    NULLIFIERS, POSEIDON,
};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cosmwasm-anchor";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// ChainType info
const COSMOS_CHAIN_TYPE: [u8; 2] = [4, 0]; // 0x0400

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

    // Initialize the Anchor_verifier
    ANCHORVERIFIER.save(deps.storage, &AnchorVerifier::new())?;

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

    // Initialize the Anchor
    let anchor = Anchor {
        chain_id: msg.chain_id,
        deposit_size: Uint256::from(msg.deposit_size.u128()),
        linkable_tree: linkable_merkle_tree,
        merkle_tree,
        cw20_address,
    };
    ANCHOR.save(deps.storage, &anchor)?;

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
        ExecuteMsg::Deposit(msg) => deposit(deps, info, msg),
        ExecuteMsg::Withdraw(msg) => withdraw(deps, info, msg),
        ExecuteMsg::Receive(msg) => receive_cw20(deps, info, msg),
    }
}

/// User deposits the fund(UST) with its commitment.
/// It checks the validity of the fund(UST) sent.
/// It also checks the merkle tree availiability.
/// It saves the commitment in "merkle tree".
pub fn deposit(
    deps: DepsMut,
    info: MessageInfo,
    msg: DepositMsg,
) -> Result<Response, ContractError> {
    let anchor = ANCHOR.load(deps.storage)?;

    // Validation 1. Check if the enough UST are sent.
    let sent_uusd: Vec<Coin> = info
        .funds
        .into_iter()
        .filter(|x| x.denom == "uusd")
        .collect();
    if sent_uusd.is_empty() || Uint256::from(sent_uusd[0].amount) < anchor.deposit_size {
        return Err(ContractError::InsufficientFunds {});
    }
    // Checks the validity of
    if let Some(commitment) = msg.commitment {
        let mut merkle_tree = anchor.merkle_tree;
        let poseidon = POSEIDON.load(deps.storage)?;
        let res = merkle_tree
            .insert(poseidon, commitment, deps.storage)
            .map_err(|_| ContractError::MerkleTreeIsFull)?;

        ANCHOR.save(
            deps.storage,
            &Anchor {
                chain_id: anchor.chain_id,
                deposit_size: anchor.deposit_size,
                linkable_tree: anchor.linkable_tree,
                cw20_address: anchor.cw20_address,
                merkle_tree,
            },
        )?;

        Ok(Response::new().add_attributes(vec![
            attr("method", "deposit"),
            attr("result", res.to_string()),
        ]))
    } else {
        Err(ContractError::Std(StdError::NotFound {
            kind: "Commitment".to_string(),
        }))
    }
}

/// User deposits the Cw20 tokens with its commitments.
/// The deposit starts from executing the hook message 
/// coming from the Cw20 token contract.
/// /// It checks the validity of the Cw20 tokens sent.
/// It also checks the merkle tree availiability.
/// It saves the commitment in "merkle tree".
pub fn receive_cw20(deps: DepsMut, info: MessageInfo, cw20_msg: Cw20ReceiveMsg) -> Result<Response, ContractError> {
    // Only Cw20 token contract can execute this message.
    let anchor: Anchor = ANCHOR.load(deps.storage)?;
    if anchor.cw20_address != deps.api.addr_canonicalize(info.sender.as_str())? {
        return Err(ContractError::Unauthorized {});
    }

    let tokens_sent = cw20_msg.amount;
    match from_binary(&cw20_msg.msg) {
        Ok(Cw20HookMsg::DepositCw20 { commitment }) => {
            if Uint256::from(tokens_sent) < anchor.deposit_size {
                return Err(ContractError::InsufficientFunds {});
            }
            // Checks the validity of
            if let Some(commitment) = commitment {
                let mut merkle_tree = anchor.merkle_tree;
                let poseidon = POSEIDON.load(deps.storage)?;
                let res = merkle_tree
                    .insert(poseidon, commitment, deps.storage)
                    .map_err(|_| ContractError::MerkleTreeIsFull)?;
        
                ANCHOR.save(
                    deps.storage,
                    &Anchor {
                        chain_id: anchor.chain_id,
                        deposit_size: anchor.deposit_size,
                        linkable_tree: anchor.linkable_tree,
                        cw20_address: anchor.cw20_address,
                        merkle_tree,
                    },
                )?;
        
                Ok(Response::new().add_attributes(vec![
                    attr("method", "deposit_cw20"),
                    attr("result", res.to_string()),
                ]))
            } else {
                Err(ContractError::Std(StdError::NotFound {
                    kind: "Commitment".to_string(),
                }))
            }
        },
        Err(_) => Err(ContractError::Std(StdError::generic_err("invalid cw20 hook msg")))
    }
}

/// User withdraws the fund(UST) to "recipient" address
/// by providing the "proof" for the "commitment".
/// It verifies the "withdraw" by verifying the "proof"
/// with "commitment" saved in prior.
/// If success on verify, then it performs "withdraw" action
/// which sends the fund(UST) to "recipient" & "relayer" address.
pub fn withdraw(
    deps: DepsMut,
    info: MessageInfo,
    msg: WithdrawMsg,
) -> Result<Response, ContractError> {
    // Validation 1. Check if the funds are sent.
    if !info.funds.is_empty() {
        return Err(ContractError::UnnecessaryFunds {});
    }

    // Validation 2. Check if the root is known to merkle tree
    let anchor = ANCHOR.load(deps.storage)?;

    let merkle_tree = anchor.merkle_tree;
    if !merkle_tree.is_known_root(msg.roots[0], deps.storage) {
        return Err(ContractError::Std(StdError::GenericErr {
            msg: "Root is not known".to_string(),
        }));
    }

    // Validation 3. Check if the roots are valid in linkable tree.
    let linkable_tree = anchor.linkable_tree;
    if !linkable_tree.is_valid_neighbor_roots(&msg.roots[1..], deps.storage) {
        return Err(ContractError::Std(StdError::GenericErr {
            msg: "Neighbor roots are not valid".to_string(),
        }));
    }

    // Checks if the nullifier already used.
    if is_known_nullifier(deps.storage, msg.nullifier_hash) {
        return Err(ContractError::Std(StdError::GenericErr {
            msg: "Nullifier is known".to_string(),
        }));
    }

    let element_encoder = |v: &[u8]| {
        let mut output = [0u8; 32];
        output.iter_mut().zip(v).for_each(|(b1, b2)| *b1 = *b2);
        output
    };

    // Format the public input bytes
    let chain_id_type_bytes =
        element_encoder(&compute_chain_id_type(anchor.chain_id, &COSMOS_CHAIN_TYPE).to_le_bytes());
    let recipient_bytes =
        truncate_and_pad(&hex::decode(&msg.recipient).map_err(|_| ContractError::DecodeError)?);
    let relayer_bytes =
        truncate_and_pad(&hex::decode(&msg.relayer).map_err(|_| ContractError::DecodeError)?);
    let fee_bytes = element_encoder(&msg.fee.to_le_bytes());
    let refund_bytes = element_encoder(&msg.refund.to_le_bytes());

    // Join the public input bytes
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&chain_id_type_bytes);
    bytes.extend_from_slice(&msg.nullifier_hash);
    for root in msg.roots {
        bytes.extend_from_slice(&root);
    }
    bytes.extend_from_slice(&recipient_bytes);
    bytes.extend_from_slice(&relayer_bytes);
    bytes.extend_from_slice(&fee_bytes);
    bytes.extend_from_slice(&refund_bytes);
    bytes.extend_from_slice(&msg.commitment);

    // Verify the proof
    let verifier = ANCHORVERIFIER.load(deps.storage)?;
    let result = verify(verifier, bytes, msg.proof_bytes)?;

    if !result {
        return Err(ContractError::Std(StdError::GenericErr {
            msg: "Invalid withdraw proof".to_string(),
        }));
    }

    // Set used nullifier to true after successful verification
    NULLIFIERS.save(deps.storage, msg.nullifier_hash.to_vec(), &true)?;

    // Send the funds
    let mut msgs: Vec<CosmosMsg> = vec![];

    // Send the funds to "recipient"
    let amt_to_recipient = match Uint128::try_from(anchor.deposit_size - msg.fee) {
        Ok(v) => v,
        Err(_) => {
            return Err(ContractError::Std(StdError::GenericErr {
                msg: "Cannot compute amount".to_string(),
            }))
        }
    };

    // Send the funds to "relayer"
    let amt_to_relayer = match Uint128::try_from(msg.fee) {
        Ok(v) => v,
        Err(_) => {
            return Err(ContractError::Std(StdError::GenericErr {
                msg: "Cannot compute amount".to_string(),
            }))
        }
    };

    // If "refund" field is non-zero, send the funds to "recipient"
    let mut amt_refund: Uint128 = Uint128::zero();
    if msg.refund > Uint256::zero() {
        amt_refund = match Uint128::try_from(msg.refund) {
            Ok(v) => v,
            Err(_) => {
                return Err(ContractError::Std(StdError::GenericErr {
                    msg: "Cannot compute amount".to_string(),
                }))
            }
        };
    }

    // If the "cw20_address" is set, then send the Cw20 tokens.
    // Otherwise, send the native tokens.
    if let Some(cw20_address) = msg.cw20_address {
        // Validate the "cw20_address".
        if anchor.cw20_address != deps.api.addr_canonicalize(cw20_address.as_str())? {
            return Err(ContractError::Std(StdError::generic_err("Invalid cw20 address")));
        }
        let cw20_token_contract = cw20_address;

        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cw20_token_contract.clone(),
            funds: [].to_vec(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: msg.recipient.clone(),
                amount: amt_to_recipient,
            })?
        }));

        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cw20_token_contract.clone(),
            funds: [].to_vec(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: msg.relayer.clone(),
                amount: amt_to_relayer,
            })?
        }));

        
        if msg.refund > Uint256::zero() {
            msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: cw20_token_contract,
                funds: [].to_vec(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: msg.recipient.clone(),
                    amount: amt_refund,
                })?
            }));
        }
    } else {
        msgs.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: msg.recipient.clone(),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: amt_to_recipient,
            }],
        }));

        msgs.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: msg.relayer,
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: amt_to_relayer,
            }],
        }));

        if msg.refund > Uint256::zero() {
            msgs.push(CosmosMsg::Bank(BankMsg::Send {
                to_address: msg.recipient,
                amount: vec![Coin {
                    denom: "uusd".to_string(),
                    amount: amt_refund,
                }],
            }));
        }
    }

    Ok(Response::new()
        .add_attributes(vec![attr("method", "withdraw")])
        .add_messages(msgs))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        // TODO
    }
}

// Check if the "nullifier" is already used or not.
fn is_known_nullifier(store: &dyn Storage, nullifier: [u8; 32]) -> bool {
    NULLIFIERS.has(store, nullifier.to_vec())
}

// Using "anchor_verifier", verifies if the "proof" really came from "public_input".
fn verify(
    verifier: AnchorVerifier,
    public_input: Vec<u8>,
    proof_bytes: Vec<u8>,
) -> Result<bool, ContractError> {
    verifier
        .verify(public_input, proof_bytes)
        .map_err(|_| ContractError::VerifyError)
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

#[cfg(test)]
mod tests {
    use crate::test_util::Element;

    use super::*;
    use ark_bn254::Fr;
    use ark_crypto_primitives::CRH as CRHTrait;
    use ark_ff::PrimeField;
    use ark_ff::{BigInteger, Field};
    use ark_std::One;
    use arkworks_gadgets::poseidon::CRH;
    use arkworks_utils::utils::bn254_x5_5::get_poseidon_bn254_x5_5;
    use arkworks_utils::utils::common::Curve;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{attr, coins, Uint128};
    type PoseidonCRH5 = CRH<Fr>;

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(&[]);

        let env = mock_env();
        let info = mock_info("anyone", &[]);
        let instantiate_msg = InstantiateMsg {
            max_edges: 0,
            chain_id: 1,
            levels: 0,
            deposit_size: Uint128::from(1_000_000_u128),
            cw20_address: "terra1fex9f78reuwhfsnc8sun6mz8rl9zwqh03fhwf3".to_string(),
        };

        // Should pass this "unwrap" if success.
        let response = instantiate(deps.as_mut(), env, info, instantiate_msg).unwrap();

        assert_eq!(
            response.attributes,
            vec![attr("method", "instantiate"), attr("owner", "anyone"),]
        );
    }

    #[test]
    fn test_deposit() {
        let mut deps = mock_dependencies(&coins(2, "token"));

        // Initialize the contract
        let env = mock_env();
        let info = mock_info("anyone", &[]);
        let instantiate_msg = InstantiateMsg {
            max_edges: 2,
            chain_id: 1,
            levels: 30,
            deposit_size: Uint128::from(1_000_000_u128),
        };

        let _ = instantiate(deps.as_mut(), env, info, instantiate_msg).unwrap();

        // Initialize the mixer
        let params = get_poseidon_bn254_x5_5();
        let left_input = Fr::one().into_repr().to_bytes_le();
        let right_input = Fr::one().double().into_repr().to_bytes_le();
        let mut input = Vec::new();
        input.extend_from_slice(&left_input);
        input.extend_from_slice(&right_input);
        let res = <PoseidonCRH5 as CRHTrait>::evaluate(&params, &input).unwrap();
        let mut element: [u8; 32] = [0u8; 32];
        element.copy_from_slice(&res.into_repr().to_bytes_le());

        // Try the deposit with insufficient fund
        let info = mock_info("depositor", &[Coin::new(1_000_u128, "uusd")]);
        let deposit_msg = DepositMsg {
            from: None,
            commitment: Some(element),
            value: Uint256::from(0_u128),
        };

        let err = deposit(deps.as_mut(), info, deposit_msg).unwrap_err();
        assert_eq!(err.to_string(), "Insufficient_funds".to_string());

        // Try the deposit with empty commitment
        let info = mock_info("depositor", &[Coin::new(1_000_000_u128, "uusd")]);
        let deposit_msg = DepositMsg {
            from: None,
            commitment: None,
            value: Uint256::from(0_u128),
        };

        let err = deposit(deps.as_mut(), info, deposit_msg).unwrap_err();
        assert_eq!(err.to_string(), "Commitment not found".to_string());

        // Try the deposit for success
        let info = mock_info("depositor", &[Coin::new(1_000_000_u128, "uusd")]);
        let deposit_msg = DepositMsg {
            from: None,
            commitment: Some(element),
            value: Uint256::from(0_u128),
        };

        let response = deposit(deps.as_mut(), info, deposit_msg).unwrap();
        assert_eq!(
            response.attributes,
            vec![attr("method", "deposit"), attr("result", "0")]
        );
    }

    #[test]
    fn test_withdraw_wasm_utils() {
        let curve = Curve::Bn254;
        let (pk_bytes, _) = crate::test_util::setup_environment(curve);
        let src_chain_id = compute_chain_id_type(1u64, &COSMOS_CHAIN_TYPE);
        let recipient_bytes = [2u8; 32];
        let relayer_bytes = [0u8; 32];
        let fee_value = 0;
        let refund_value = 0;
        let commitment_bytes = [0u8; 32];
        let commitment_element = Element::from_bytes(&commitment_bytes);

        // Setup zk circuit for withdraw
        let (proof_bytes, root_elements, nullifier_hash_element, leaf_element) =
            crate::test_util::setup_wasm_utils_zk_circuit(
                curve,
                truncate_and_pad(&recipient_bytes),
                truncate_and_pad(&relayer_bytes),
                commitment_bytes,
                pk_bytes.clone(),
                src_chain_id as u128,
                fee_value,
                refund_value,
            );

        let mut deps = mock_dependencies(&coins(2, "token"));

        // Initialize the contract
        let env = mock_env();
        let info = mock_info("anyone", &[]);
        let instantiate_msg = InstantiateMsg {
            max_edges: 2,
            chain_id: 1,
            levels: 30,
            deposit_size: Uint128::from(1_000_000_u128),
        };

        let _ = instantiate(deps.as_mut(), env, info, instantiate_msg).unwrap();

        // Try the deposit for success
        let info = mock_info("depositor", &[Coin::new(1_000_000_u128, "uusd")]);
        let deposit_msg = DepositMsg {
            from: None,
            commitment: Some(leaf_element.0),
            value: Uint256::from(0_u128),
        };

        let response = deposit(deps.as_mut(), info, deposit_msg.clone()).unwrap();
        assert_eq!(
            response.attributes,
            vec![attr("method", "deposit"), attr("result", "0")]
        );
        let on_chain_root = crate::state::read_root(&deps.storage, 1).unwrap();
        let local_root = root_elements[0].0;
        assert_eq!(on_chain_root, local_root);

        // Invalid withdraw proof leads to failure result.
        let mut false_proof_bytes = proof_bytes.clone();
        false_proof_bytes[0] = 0;
        false_proof_bytes[1] = 0;
        false_proof_bytes[2] = 0;

        let mut roots = vec![];
        for i in 0..root_elements.len() {
            roots.push(root_elements[i].0);
        }

        let withdraw_msg = WithdrawMsg {
            proof_bytes: false_proof_bytes,
            roots: roots,
            nullifier_hash: nullifier_hash_element.0,
            recipient: hex::encode(recipient_bytes.to_vec()),
            relayer: hex::encode(relayer_bytes.to_vec()),
            fee: cosmwasm_std::Uint256::from(fee_value),
            refund: cosmwasm_std::Uint256::from(refund_value),
            commitment: commitment_element.0,
        };
        let info = mock_info("withdraw", &[]);
        let err = withdraw(deps.as_mut(), info, withdraw_msg).unwrap_err();
        assert_eq!(err.to_string(), "VerifyError".to_string());

        // Should succeed
        let mut roots = vec![];
        for i in 0..root_elements.len() {
            roots.push(root_elements[i].0);
        }

        let withdraw_msg = WithdrawMsg {
            proof_bytes: proof_bytes,
            roots: roots,
            nullifier_hash: nullifier_hash_element.0,
            recipient: hex::encode(recipient_bytes.to_vec()),
            relayer: hex::encode(relayer_bytes.to_vec()),
            fee: cosmwasm_std::Uint256::from(fee_value),
            refund: cosmwasm_std::Uint256::from(refund_value),
            commitment: commitment_element.0,
        };
        let info = mock_info("withdraw", &[]);
        let response = withdraw(deps.as_mut(), info, withdraw_msg).unwrap();
        assert_eq!(response.attributes, vec![attr("method", "withdraw")]);
    }

    #[test]
    fn test_withdraw_native() {
        let curve = Curve::Bn254;
        let (pk_bytes, _) = crate::test_util::setup_environment(curve);
        let recipient_bytes = [1u8; 32];
        let relayer_bytes = [2u8; 32];
        let fee_value = 0;
        let refund_value = 0;
        let src_chain_id = compute_chain_id_type(1, &COSMOS_CHAIN_TYPE);
        let commitment_bytes = vec![0u8; 32];
        let commit_element = Element::from_bytes(&commitment_bytes);

        // Setup zk circuit for withdraw
        let (proof_bytes, root_elements, nullifier_hash_element, leaf_element) =
            crate::test_util::setup_zk_circuit(
                curve,
                truncate_and_pad(&recipient_bytes),
                truncate_and_pad(&relayer_bytes),
                commitment_bytes.clone(),
                pk_bytes.clone(),
                src_chain_id,
                fee_value,
                refund_value,
            );

        let mut deps = mock_dependencies(&coins(2, "token"));

        // Initialize the contract
        let env = mock_env();
        let info = mock_info("anyone", &[]);
        let instantiate_msg = InstantiateMsg {
            max_edges: 2,
            chain_id: 1,
            levels: 30,
            deposit_size: Uint128::from(1_000_000_u128),
        };

        let _ = instantiate(deps.as_mut(), env, info, instantiate_msg).unwrap();

        // Try the deposit for success
        let info = mock_info("depositor", &[Coin::new(1_000_000_u128, "uusd")]);
        let deposit_msg = DepositMsg {
            from: None,
            commitment: Some(leaf_element.0),
            value: Uint256::from(0_u128),
        };

        let response = deposit(deps.as_mut(), info, deposit_msg.clone()).unwrap();
        assert_eq!(
            response.attributes,
            vec![attr("method", "deposit"), attr("result", "0")]
        );
        let on_chain_root = crate::state::read_root(&deps.storage, 1).unwrap();
        let local_root = root_elements[0].0;
        assert_eq!(on_chain_root, local_root);

        // Invalid root_element leads to failure.
        let mut false_roots = vec![];
        for i in 0..root_elements.len() {
            false_roots.push(root_elements[i].0);
        }
        false_roots[0][0] = 0;

        let withdraw_msg = WithdrawMsg {
            proof_bytes: proof_bytes.clone(),
            roots: false_roots,
            nullifier_hash: nullifier_hash_element.0,
            recipient: hex::encode(recipient_bytes.to_vec()),
            relayer: hex::encode(relayer_bytes.to_vec()),
            fee: cosmwasm_std::Uint256::from(fee_value),
            refund: cosmwasm_std::Uint256::from(refund_value),
            commitment: commit_element.0,
        };
        let info = mock_info("withdraw", &[]);
        let err = withdraw(deps.as_mut(), info, withdraw_msg).unwrap_err();
        assert_eq!(
            err.to_string(),
            "Generic error: Root is not known".to_string()
        );

        // Should succeed
        let mut roots = vec![];
        for elem in root_elements {
            roots.push(elem.0);
        }
        let withdraw_msg = WithdrawMsg {
            proof_bytes: proof_bytes,
            roots: roots,
            nullifier_hash: nullifier_hash_element.0,
            recipient: hex::encode(recipient_bytes.to_vec()),
            relayer: hex::encode(relayer_bytes.to_vec()),
            fee: cosmwasm_std::Uint256::from(fee_value),
            refund: cosmwasm_std::Uint256::from(refund_value),
            commitment: commit_element.0,
        };
        let info = mock_info("withdraw", &[]);
        let response = withdraw(deps.as_mut(), info, withdraw_msg).unwrap();
        assert_eq!(response.attributes, vec![attr("method", "withdraw")]);
    }
}
