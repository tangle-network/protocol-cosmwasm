#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response, StdError,
    StdResult, Storage, Uint128, Uint256,
};
use cw2::set_contract_version;
use std::convert::TryFrom;

use crate::error::ContractError;
use crate::mixer_verifier::MixerVerifier;
use crate::msg::{DepositMsg, ExecuteMsg, InstantiateMsg, QueryMsg, WithdrawMsg};
use crate::poseidon::Poseidon;
use crate::state::{
    read_root, save_root, save_subtree, MerkleTree, Mixer, MIXER, MIXERVERIFIER, NULLIFIERS,
    POSEIDON,
};
use crate::zeroes::zeroes;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cosmwasm-mixer";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    // Validation 1. Check if the funds are sent with this message
    if !info.funds.is_empty() {
        return Err(ContractError::UnnecessaryFunds {});
    }

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Initialize the merkle tree
    let merkle_tree: MerkleTree = MerkleTree {
        levels: msg.merkletree_levels,
        current_root_index: 0,
        next_index: 0,
    };
    // Initialize the Mixer
    let mixer: Mixer = Mixer {
        initialized: true,
        deposit_size: Uint256::from(msg.deposit_size.u128()),
        merkle_tree,
    };
    MIXER.save(deps.storage, &mixer)?;

    // Initialize the poseidon hasher
    POSEIDON.save(deps.storage, &Poseidon::new())?;

    // Initialize the Mixer_Verifier
    MIXERVERIFIER.save(deps.storage, &MixerVerifier::new())?;

    for i in 0..msg.merkletree_levels {
        save_subtree(deps.storage, i as u32, &zeroes(i))?;
    }

    save_root(deps.storage, 0_u32, &zeroes(msg.merkletree_levels))?;

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
    }
}

pub fn deposit(
    deps: DepsMut,
    info: MessageInfo,
    msg: DepositMsg,
) -> Result<Response, ContractError> {
    let mixer = MIXER.load(deps.storage)?;

    // Validation 1. Check if the enough UST are sent.
    let sent_uusd: Vec<Coin> = info
        .funds
        .into_iter()
        .filter(|x| x.denom == "uusd")
        .collect();
    if sent_uusd.is_empty() || Uint256::from(sent_uusd[0].amount) < mixer.deposit_size {
        return Err(ContractError::InsufficientFunds {});
    }

    // Validation 2. Check if the mixer is initialized
    if !mixer.initialized {
        return Err(ContractError::NotInitialized {});
    }

    if let Some(commitment) = msg.commitment {
        let mut merkle_tree = mixer.merkle_tree;
        let poseidon = POSEIDON.load(deps.storage)?;
        // let poseidon = Poseidon::new();
        let res = merkle_tree.insert(poseidon, commitment, deps.storage)?;
        MIXER.save(
            deps.storage,
            &Mixer {
                initialized: mixer.initialized,
                deposit_size: mixer.deposit_size,
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
    let mixer = MIXER.load(deps.storage)?;
    let merkle_tree = mixer.merkle_tree;

    if !merkle_tree.is_known_root(msg.root, deps.storage) {
        return Err(ContractError::Std(StdError::GenericErr {
            msg: "Root is not known".to_string(),
        }));
    }

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
    let recipient_bytes =
        truncate_and_pad(&hex::decode(&msg.recipient).map_err(|_| ContractError::DecodeError)?);
    let relayer_bytes =
        truncate_and_pad(&hex::decode(&msg.relayer).map_err(|_| ContractError::DecodeError)?);
    let fee_bytes = element_encoder(&msg.fee.to_be_bytes());
    let refund_bytes = element_encoder(&msg.refund.to_be_bytes());

    // Join the public input bytes
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&msg.nullifier_hash);
    bytes.extend_from_slice(&msg.root);
    bytes.extend_from_slice(&recipient_bytes);
    bytes.extend_from_slice(&relayer_bytes);
    bytes.extend_from_slice(&fee_bytes);
    bytes.extend_from_slice(&refund_bytes);
    // Verify the proof
    let verifier = MIXERVERIFIER.load(deps.storage)?;
    let result = verify(verifier, bytes, msg.proof_bytes)?;

    if !result {
        return Err(ContractError::Std(StdError::GenericErr {
            msg: "Invalid withdraw proof".to_string(),
        }));
    }

    // Set used nullifier to true after successful verification
    NULLIFIERS.save(deps.storage, msg.nullifier_hash.to_vec(), &true)?;

    // Send the funds
    // TODO: Support "ERC20"-like tokens
    let mut msgs: Vec<CosmosMsg> = vec![];

    let amt = match Uint128::try_from(mixer.deposit_size - msg.fee) {
        Ok(v) => v,
        Err(_) => {
            return Err(ContractError::Std(StdError::GenericErr {
                msg: "Cannot compute amount".to_string(),
            }))
        }
    };
    msgs.push(CosmosMsg::Bank(BankMsg::Send {
        to_address: msg.recipient.clone(),
        amount: vec![Coin {
            denom: "uusd".to_string(),
            amount: amt,
        }],
    }));

    let amt = match Uint128::try_from(msg.fee) {
        Ok(v) => v,
        Err(_) => {
            return Err(ContractError::Std(StdError::GenericErr {
                msg: "Cannot compute amount".to_string(),
            }))
        }
    };
    msgs.push(CosmosMsg::Bank(BankMsg::Send {
        to_address: msg.relayer,
        amount: vec![Coin {
            denom: "uusd".to_string(),
            amount: amt,
        }],
    }));

    if msg.refund > Uint256::zero() {
        let amt = match Uint128::try_from(msg.refund) {
            Ok(v) => v,
            Err(_) => {
                return Err(ContractError::Std(StdError::GenericErr {
                    msg: "Cannot compute amount".to_string(),
                }))
            }
        };
        msgs.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: msg.recipient,
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: amt,
            }],
        }));
    }

    Ok(Response::new()
        .add_attributes(vec![attr("method", "withdraw")])
        .add_messages(msgs))
}

fn is_known_nullifier(store: &dyn Storage, nullifier: [u8; 32]) -> bool {
    NULLIFIERS.has(store, nullifier.to_vec())
}

fn verify(
    verifier: MixerVerifier,
    public_input: Vec<u8>,
    proof_bytes: Vec<u8>,
) -> Result<bool, ContractError> {
    verifier
        .verify(public_input, proof_bytes)
        .map_err(|_| ContractError::VerifyError)
}

fn truncate_and_pad(t: &[u8]) -> Vec<u8> {
    let mut truncated_bytes = t[..20].to_vec();
    truncated_bytes.extend_from_slice(&[0u8; 12]);
    truncated_bytes
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        // TODO
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_bn254::Fr;
    use ark_crypto_primitives::CRH as CRHTrait;
    use ark_ff::PrimeField;
    use ark_ff::{BigInteger, Field};
    use ark_std::One;
    use arkworks_gadgets::merkle_tree::simple_merkle::gen_empty_hashes;
    use arkworks_gadgets::poseidon::CRH;
    use arkworks_utils::utils::bn254_x5_5::get_poseidon_bn254_x5_5;
    use arkworks_utils::utils::common::{setup_params_x5_5, Curve};
    use arkworks_utils::utils::parse_vec;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{attr, coins, Uint128};
    type PoseidonCRH5 = CRH<Fr>;
    use arkworks_gadgets::poseidon::field_hasher;

    fn get_empty_hashes(curve: Curve) {
        let params = setup_params_x5_5::<Fr>(curve);
        let poseidon = field_hasher::Poseidon::<Fr>::new(params.clone());

        let default_leaf_hex =
            vec!["0x2fe54c60d3acabf3343a35b6eba15db4821b340f76e741e2249685ed4899af6c"];
        let default_leaf_scalar: Vec<Fr> = parse_vec(default_leaf_hex);
        let default_leaf_vec = default_leaf_scalar[0].into_repr().to_bytes_le();
        let empty_hashes =
            gen_empty_hashes::<Fr, _, 32usize>(&poseidon, &default_leaf_vec[..]).unwrap();
    }

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(&[]);

        let env = mock_env();
        let info = mock_info("anyone", &[]);
        let instantiate_msg = InstantiateMsg {
            merkletree_levels: 0,
            deposit_size: Uint128::from(1_000_000_u128),
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
            merkletree_levels: 30,
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
        let recipient_bytes = [1u8; 32];
        let relayer_bytes = [2u8; 32];
        let fee_value = 0;
        let refund_value = 0;
        // Setup zk circuit for withdraw
        let (mut proof_bytes, root_element, nullifier_hash_element, leaf_element) =
            crate::test_util::setup_wasm_utils_zk_circuit(
                curve,
                truncate_and_pad(&recipient_bytes),
                truncate_and_pad(&relayer_bytes),
                pk_bytes.clone(),
                fee_value,
                refund_value,
            );
        let mut deps = mock_dependencies(&coins(2, "token"));

        // Initialize the contract
        let env = mock_env();
        let info = mock_info("anyone", &[]);
        let instantiate_msg = InstantiateMsg {
            merkletree_levels: 30,
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
        let on_chain_root = read_root(&deps.storage, 1).unwrap();
        let local_root = root_element.0;
        assert_eq!(on_chain_root, local_root);

        // Invalid withdraw proof leads to failure result.
        proof_bytes[0] = 0;

        let withdraw_msg = WithdrawMsg {
            proof_bytes: proof_bytes,
            root: root_element.0,
            nullifier_hash: nullifier_hash_element.0,
            recipient: hex::encode(recipient_bytes.to_vec()),
            relayer: hex::encode(relayer_bytes.to_vec()),
            fee: cosmwasm_std::Uint256::from(fee_value),
            refund: cosmwasm_std::Uint256::from(refund_value),
        };
        let info = mock_info("withdraw", &[]);
        let err = withdraw(deps.as_mut(), info, withdraw_msg).unwrap_err();
        assert_eq!(err.to_string(), "Generic error: Invalid withdraw proof".to_string());

        let (proof_bytes, root_element, nullifier_hash_element, _leaf_element) =
            crate::test_util::setup_wasm_utils_zk_circuit(
                curve,
                truncate_and_pad(&recipient_bytes),
                truncate_and_pad(&relayer_bytes),
                pk_bytes,
                fee_value,
                refund_value,
            );
        let withdraw_msg = WithdrawMsg {
            proof_bytes: proof_bytes,
            root: root_element.0,
            nullifier_hash: nullifier_hash_element.0,
            recipient: hex::encode(recipient_bytes.to_vec()),
            relayer: hex::encode(relayer_bytes.to_vec()),
            fee: cosmwasm_std::Uint256::from(fee_value),
            refund: cosmwasm_std::Uint256::from(refund_value),
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
        // Setup zk circuit for withdraw
        let (proof_bytes, mut root_element, nullifier_hash_element, leaf_element) =
            crate::test_util::setup_zk_circuit(
                curve,
                truncate_and_pad(&recipient_bytes),
                truncate_and_pad(&relayer_bytes),
                pk_bytes.clone(),
                fee_value,
                refund_value,
            );

        let mut deps = mock_dependencies(&coins(2, "token"));

        // Initialize the contract
        let env = mock_env();
        let info = mock_info("anyone", &[]);
        let instantiate_msg = InstantiateMsg {
            merkletree_levels: 30,
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
        let on_chain_root = read_root(&deps.storage, 1).unwrap();
        let local_root = root_element.0;
        assert_eq!(on_chain_root, local_root);

        // Invalid root_element leads to failure.
        root_element.0[0] = 0;
        let withdraw_msg = WithdrawMsg {
            proof_bytes: proof_bytes,
            root: root_element.0,
            nullifier_hash: nullifier_hash_element.0,
            recipient: hex::encode(recipient_bytes.to_vec()),
            relayer: hex::encode(relayer_bytes.to_vec()),
            fee: cosmwasm_std::Uint256::from(fee_value),
            refund: cosmwasm_std::Uint256::from(refund_value),
        };
        let info = mock_info("withdraw", &[]);
        let err = withdraw(deps.as_mut(), info, withdraw_msg).unwrap_err();
        assert_eq!(
            err.to_string(),
            "Generic error: Root is not known".to_string()
        );

        let (proof_bytes, root_element, nullifier_hash_element, leaf_element) =
            crate::test_util::setup_zk_circuit(
                curve,
                truncate_and_pad(&recipient_bytes),
                truncate_and_pad(&relayer_bytes),
                pk_bytes,
                fee_value,
                refund_value,
            );

        let withdraw_msg = WithdrawMsg {
            proof_bytes: proof_bytes,
            root: root_element.0,
            nullifier_hash: nullifier_hash_element.0,
            recipient: hex::encode(recipient_bytes.to_vec()),
            relayer: hex::encode(relayer_bytes.to_vec()),
            fee: cosmwasm_std::Uint256::from(fee_value),
            refund: cosmwasm_std::Uint256::from(refund_value),
        };
        let info = mock_info("withdraw", &[]);
        let response = withdraw(deps.as_mut(), info, withdraw_msg).unwrap();
        assert_eq!(response.attributes, vec![attr("method", "withdraw")]);
    }
}
