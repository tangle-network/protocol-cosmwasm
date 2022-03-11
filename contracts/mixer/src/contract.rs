#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, from_binary, attr, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response, StdError,
    StdResult, Storage, Uint128, Uint256, WasmMsg
};
use cw2::set_contract_version;
use std::convert::TryFrom;

use protocol_cosmwasm::error::ContractError;
use protocol_cosmwasm::mixer::{DepositMsg, ExecuteMsg, InstantiateMsg, QueryMsg, WithdrawMsg, Cw20HookMsg, InfoResponse};
use protocol_cosmwasm::mixer_verifier::MixerVerifier;
use protocol_cosmwasm::poseidon::Poseidon;
use protocol_cosmwasm::zeroes::zeroes;

use cw20::{Cw20ReceiveMsg, Cw20ExecuteMsg};

use crate::state::{
    save_root, save_subtree, MerkleTree, Mixer, MIXER, MIXERVERIFIER, USED_NULLIFIERS, POSEIDON,
};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cosmwasm-mixer";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// User instantiates the "mixer" contract.
/// IMPORTANT:
///     Every individual mixer is for either native(UST) token or CW20 token.
///     For example, when instantiating:
///         If the "cw20_address" field is empty, then the mixer is for native(UST) token. 
///         If the "cw20_address" field is set,   then the mixer is for CW20 token.
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

    // Check the validity of "cw20_address" if exists.
    let cw20_address = match msg.cw20_address {
        Some(addr) => Some(deps.api.addr_canonicalize(addr.as_str())?),
        None => None
    };

    // Initialize the Mixer
    let mixer: Mixer = Mixer {
        initialized: true,
        deposit_size: Uint256::from(msg.deposit_size.u128()),
        merkle_tree,
        cw20_address,
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

    // Validation 3. Check if the mixer is for native(UST) token.
    if mixer.cw20_address.is_some() {
        return Err(ContractError::Std(StdError::generic_err("This mixer is for CW20 token")));
    }

    if let Some(commitment) = msg.commitment {
        let mut merkle_tree = mixer.merkle_tree;
        let poseidon = POSEIDON.load(deps.storage)?;
        let res = merkle_tree.insert(poseidon, commitment, deps.storage)?;
        MIXER.save(
            deps.storage,
            &Mixer {
                initialized: mixer.initialized,
                deposit_size: mixer.deposit_size,
                cw20_address: mixer.cw20_address,
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
/// It checks the validity of the Cw20 tokens sent.
/// It also checks the merkle tree availiability.
/// It saves the commitment in "merkle tree".
pub fn receive_cw20(
    deps: DepsMut,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    // Only Cw20 token contract can execute this message.
    let mixer: Mixer = MIXER.load(deps.storage)?;
    let cw20_address = mixer.cw20_address.clone();
    if cw20_address.is_none() {
        return Err(ContractError::Std(StdError::generic_err("This mixer is for native(UST) token")));
    }

    if cw20_address.unwrap() != deps.api.addr_canonicalize(info.sender.as_str())? {
        return Err(ContractError::Unauthorized {});
    }

    let tokens_sent = cw20_msg.amount;
    match from_binary(&cw20_msg.msg) {
        Ok(Cw20HookMsg::DepositCw20 { commitment }) => {
            if Uint256::from(tokens_sent) < mixer.deposit_size {
                return Err(ContractError::InsufficientFunds {});
            }
            // Checks the validity of
            if let Some(commitment) = commitment {
                let mut merkle_tree = mixer.merkle_tree;
                let poseidon = POSEIDON.load(deps.storage)?;
                let res = merkle_tree
                    .insert(poseidon, commitment, deps.storage)
                    .map_err(|_| ContractError::MerkleTreeIsFull)?;

                MIXER.save(
                    deps.storage,
                    &Mixer {
                        initialized: mixer.initialized,
                        deposit_size: mixer.deposit_size,
                        cw20_address: mixer.cw20_address,
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
        }
        Err(_) => Err(ContractError::Std(StdError::generic_err(
            "invalid cw20 hook msg",
        ))),
    }
}

/// User withdraws the native(UST) token or CW20 token 
/// to "recipient" address by providing the "proof" for 
/// the "commitment".
/// It verifies the "withdraw" by verifying the "proof"
/// with "commitment" saved in prior.
/// If success on verify, then it performs "withdraw" action
/// which sends the native(UST) token or CW20 token 
/// to "recipient" & "relayer" address.
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
    let fee_bytes = element_encoder(&msg.fee.to_le_bytes());
    let refund_bytes = element_encoder(&msg.refund.to_le_bytes());

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
    USED_NULLIFIERS.save(deps.storage, msg.nullifier_hash.to_vec(), &true)?;

    // Send the funds
    let mut msgs: Vec<CosmosMsg> = vec![];

   // Send the funds to "recipient"
   let amt_to_recipient = match Uint128::try_from(mixer.deposit_size - msg.fee) {
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
        if mixer.cw20_address.unwrap() != deps.api.addr_canonicalize(cw20_address.as_str())? {
            return Err(ContractError::Std(StdError::generic_err(
                "Invalid cw20 address",
            )));
        }
        let cw20_token_contract = cw20_address;

        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cw20_token_contract.clone(),
            funds: [].to_vec(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: msg.recipient.clone(),
                amount: amt_to_recipient,
            })?,
        }));

        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cw20_token_contract.clone(),
            funds: [].to_vec(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: msg.relayer.clone(),
                amount: amt_to_relayer,
            })?,
        }));

        if msg.refund > Uint256::zero() {
            msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: cw20_token_contract,
                funds: [].to_vec(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: msg.recipient.clone(),
                    amount: amt_refund,
                })?,
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

fn is_known_nullifier(store: &dyn Storage, nullifier: [u8; 32]) -> bool {
    USED_NULLIFIERS.has(store, nullifier.to_vec())
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
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetCw20Address {} => to_binary(&get_cw20_address(deps)?),
    }
}

fn get_cw20_address(deps: Deps) -> StdResult<InfoResponse> {
    let mixer = MIXER.load(deps.storage)?;
    
    let cw20_address = match mixer.cw20_address {
        Some(cw20_address) => deps.api.addr_humanize(&cw20_address)?.to_string(),
        None => "".to_string(),
    };

    Ok(InfoResponse {
        cw20_address, 
    })
}

#[cfg(test)]
mod tests {
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
    fn test_mixer_proper_initialization() {
        let mut deps = mock_dependencies(&[]);

        let env = mock_env();
        let info = mock_info("anyone", &[]);
        let instantiate_msg = InstantiateMsg {
            merkletree_levels: 0,
            deposit_size: Uint128::from(1_000_000_u128),
            cw20_address: None,
        };

        // Should pass this "unwrap" if success.
        let response = instantiate(deps.as_mut(), env, info, instantiate_msg).unwrap();

        assert_eq!(
            response.attributes,
            vec![attr("method", "instantiate"), attr("owner", "anyone"),]
        );
    }

    #[test]
    fn test_mixer_deposit_native_token() {
        let mut deps = mock_dependencies(&coins(2, "token"));

        // Initialize the contract
        let env = mock_env();
        let info = mock_info("anyone", &[]);
        let instantiate_msg = InstantiateMsg {
            merkletree_levels: 30,
            deposit_size: Uint128::from(1_000_000_u128),
            cw20_address: None,
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
    fn test_mixer_deposit_cw20() {
        let cw20_address = "terra1fex9f78reuwhfsnc8sun6mz8rl9zwqh03fhwf3".to_string();

        let mut deps = mock_dependencies(&coins(2, "token"));

        // Initialize the contract
        let env = mock_env();
        let info = mock_info("anyone", &[]);
        let instantiate_msg = InstantiateMsg {
            merkletree_levels: 30,
            deposit_size: Uint128::from(1_000_000_u128),
            cw20_address: Some(cw20_address.clone()),
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

        // Try the deposit for success
        let info = mock_info(cw20_address.as_str(), &[]);
        let deposit_cw20_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: cw20_address.clone(),
            amount: Uint128::from(1_000_000_u128),
            msg: to_binary(&Cw20HookMsg::DepositCw20 {
                commitment: Some(element),
            })
            .unwrap(),
        });

        let response = execute(deps.as_mut(), mock_env(), info, deposit_cw20_msg).unwrap();
        assert_eq!(
            response.attributes,
            vec![attr("method", "deposit_cw20"), attr("result", "0")]
        );
    }

    #[test]
    fn test_mixer_withdraw_native_wasm_utils() {
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
            cw20_address: None,
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
        let local_root = root_element.0;
        assert_eq!(on_chain_root, local_root);

        // Invalid withdraw proof leads to failure result.
        proof_bytes[0] = 1;

        let withdraw_msg = WithdrawMsg {
            proof_bytes: proof_bytes,
            root: root_element.0,
            nullifier_hash: nullifier_hash_element.0,
            recipient: hex::encode(recipient_bytes.to_vec()),
            relayer: hex::encode(relayer_bytes.to_vec()),
            fee: cosmwasm_std::Uint256::from(fee_value),
            refund: cosmwasm_std::Uint256::from(refund_value),
            cw20_address: None,
        };
        let info = mock_info("withdraw", &[]);
        assert!(
            withdraw(deps.as_mut(), info, withdraw_msg).is_err(),
            "Should fail with wrong proof bytes"
        );

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
            cw20_address: None,
        };
        let info = mock_info("withdraw", &[]);
        let response = withdraw(deps.as_mut(), info, withdraw_msg).unwrap();
        assert_eq!(response.attributes, vec![attr("method", "withdraw")]);
    }

    #[test]
    fn test_mixer_withdraw_native() {
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
            cw20_address: None,
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
            cw20_address: None,
        };
        let info = mock_info("withdraw", &[]);
        let err = withdraw(deps.as_mut(), info, withdraw_msg).unwrap_err();
        assert_eq!(
            err.to_string(),
            "Generic error: Root is not known".to_string()
        );

        let (proof_bytes, root_element, nullifier_hash_element, _) =
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
            cw20_address: None,
        };
        let info = mock_info("withdraw", &[]);
        let response = withdraw(deps.as_mut(), info, withdraw_msg).unwrap();
        assert_eq!(response.attributes, vec![attr("method", "withdraw")]);
    }

    #[test]
    fn test_mixer_withdraw_cw20() {
        let curve = Curve::Bn254;
        let (pk_bytes, _) = crate::test_util::setup_environment(curve);
        let recipient_bytes = [1u8; 32];
        let relayer_bytes = [2u8; 32];
        let fee_value = 0;
        let refund_value = 0;
        // Setup zk circuit for withdraw
        let (proof_bytes, root_element, nullifier_hash_element, leaf_element) =
            crate::test_util::setup_zk_circuit(
                curve,
                truncate_and_pad(&recipient_bytes),
                truncate_and_pad(&relayer_bytes),
                pk_bytes.clone(),
                fee_value,
                refund_value,
            );

        let cw20_address = "terra1fex9f78reuwhfsnc8sun6mz8rl9zwqh03fhwf3".to_string();

        let mut deps = mock_dependencies(&coins(2, "token"));

        // Initialize the contract
        let env = mock_env();
        let info = mock_info("anyone", &[]);
        let instantiate_msg = InstantiateMsg {
            merkletree_levels: 30,
            deposit_size: Uint128::from(1_000_000_u128),
            cw20_address: Some(cw20_address.clone()),
        };

        let _ = instantiate(deps.as_mut(), env, info, instantiate_msg).unwrap();

        // Try the deposit for success
        let info = mock_info(cw20_address.as_str(), &[]);
        let deposit_cw20_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: cw20_address.clone(),
            amount: Uint128::from(1_000_000_u128),
            msg: to_binary(&Cw20HookMsg::DepositCw20 {
                commitment: Some(leaf_element.0),
            })
            .unwrap(),
        });

        let response = execute(deps.as_mut(), mock_env(), info, deposit_cw20_msg).unwrap();
        assert_eq!(
            response.attributes,
            vec![attr("method", "deposit_cw20"), attr("result", "0")]
        );

        let on_chain_root = crate::state::read_root(&deps.storage, 1).unwrap();
        let local_root = root_element.0;
        assert_eq!(on_chain_root, local_root);

        // Withdraw should succeed
        let withdraw_msg = WithdrawMsg {
            proof_bytes: proof_bytes,
            root: root_element.0,
            nullifier_hash: nullifier_hash_element.0,
            recipient: hex::encode(recipient_bytes.to_vec()),
            relayer: hex::encode(relayer_bytes.to_vec()),
            fee: cosmwasm_std::Uint256::from(fee_value),
            refund: cosmwasm_std::Uint256::from(refund_value),
            cw20_address: None,
        };
        let info = mock_info("withdraw", &[]);
        let response = withdraw(deps.as_mut(), info, withdraw_msg).unwrap();
        assert_eq!(response.attributes, vec![attr("method", "withdraw")]);

        let expected_recipient = hex::encode(recipient_bytes.to_vec());
        let expected_relayer = hex::encode(relayer_bytes.to_vec());
        let expected_messages: Vec<CosmosMsg> = vec![
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: cw20_address.clone(),
                funds: [].to_vec(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: expected_recipient,
                    amount: Uint128::from(1_000_000_u128),
                })
                .unwrap(),
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: cw20_address.clone(),
                funds: [].to_vec(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: expected_relayer,
                    amount: Uint128::from(0_u128),
                })
                .unwrap(),
            }),
        ];
        assert_eq!(response.messages.len(), expected_messages.len());
    }
}
