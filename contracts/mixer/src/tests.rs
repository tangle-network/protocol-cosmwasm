use ark_bn254::Fr;
use ark_ff::BigInteger;
use ark_ff::PrimeField;
use ark_std::One;
use arkworks_native_gadgets::poseidon::FieldHasher;
use arkworks_native_gadgets::poseidon::Poseidon;
use arkworks_setups::common::setup_params;
use arkworks_setups::Curve;

use cosmwasm_std::testing::{
    mock_dependencies, mock_env, mock_info, MockApi, MockQuerier, MockStorage,
};
use cosmwasm_std::{attr, to_binary, Coin, CosmosMsg, OwnedDeps, Uint128, WasmMsg};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};

use crate::contract::{execute, instantiate, truncate_and_pad};
use crate::test_util::Element;
use protocol_cosmwasm::mixer::{Cw20HookMsg, DepositMsg, ExecuteMsg, InstantiateMsg, WithdrawMsg};

const MERKLE_TREE_LEVELS: u32 = 30;
const DEPOSIT_SIZE: &str = "1000000";
const CW20_ADDRESS: &str = "terra1fex9f78reuwhfsnc8sun6mz8rl9zwqh03fhwf3";
const NATIVE_TOKEN_DENOM: &str = "uusd";

const RECIPIENT: &str = "terra1kejftqzx05y9rv00lw5m76csfmx7lf9se02dz4";
const RELAYER: &str = "terra1jrj2vh6cstqwk3pg8nkmdf0r9z0n3q3f3jk5xn";
const FEE: u128 = 0;
const REFUND: u128 = 0;

#[derive(Debug, PartialEq)]
pub enum MixerType {
    Native,
    Cw20,
}

fn create_mixer(ty: MixerType) -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
    let mut deps = mock_dependencies(&[]);

    // Initialize the contract
    let env = mock_env();
    let info = mock_info("anyone", &[]);
    let instantiate_msg = InstantiateMsg {
        merkletree_levels: MERKLE_TREE_LEVELS,
        deposit_size: DEPOSIT_SIZE.to_string(),
        cw20_address: if ty == MixerType::Cw20 {
            Some(CW20_ADDRESS.to_string())
        } else {
            None
        },
        native_token_denom: if ty == MixerType::Native {
            Some(NATIVE_TOKEN_DENOM.to_string())
        } else {
            None
        },
    };

    let _ = instantiate(deps.as_mut(), env, info, instantiate_msg).unwrap();

    deps
}

fn prepare_wasm_utils_zk_circuit(
    curve: Curve,
    recipient: &str,
    relayer: &str,
    fee: u128,
    refund: u128,
) -> (Vec<u8>, Element, Element, Element) {
    let (pk_bytes, _) = crate::test_util::setup_environment(curve);
    let recipient_bytes = recipient.as_bytes();
    let relayer_bytes = relayer.as_bytes();
    let fee_value = fee;
    let refund_value = refund;

    // Setup zk circuit for withdraw
    crate::test_util::setup_wasm_utils_zk_circuit(
        curve,
        truncate_and_pad(recipient_bytes),
        truncate_and_pad(relayer_bytes),
        pk_bytes.clone(),
        fee_value,
        refund_value,
    )
}

fn prepare_zk_circuit(
    curve: Curve,
    recipient: &str,
    relayer: &str,
    fee: u128,
    refund: u128,
) -> (Vec<u8>, Element, Element, Element) {
    let (pk_bytes, _) = crate::test_util::setup_environment(curve);
    let recipient_bytes = recipient.as_bytes();
    let relayer_bytes = relayer.as_bytes();
    let fee_value = fee;
    let refund_value = refund;

    // Setup zk circuit for withdraw
    crate::test_util::setup_zk_circuit(
        curve,
        truncate_and_pad(recipient_bytes),
        truncate_and_pad(relayer_bytes),
        pk_bytes.clone(),
        fee_value,
        refund_value,
    )
}

#[test]
fn test_mixer_proper_initialization() {
    let mut deps = mock_dependencies(&[]);

    let env = mock_env();
    let info = mock_info("anyone", &[]);
    let instantiate_msg = InstantiateMsg {
        merkletree_levels: MERKLE_TREE_LEVELS,
        deposit_size: DEPOSIT_SIZE.to_string(),
        native_token_denom: Some(NATIVE_TOKEN_DENOM.to_string()),
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
fn test_mixer_should_be_able_to_deposit_native_token() {
    let mut deps = create_mixer(MixerType::Native);

    // Initialize the mixer
    let params = setup_params(Curve::Bn254, 5, 3);
    let poseidon = Poseidon::new(params);
    let res = poseidon.hash_two(&Fr::one(), &Fr::one()).unwrap();
    let mut element: [u8; 32] = [0u8; 32];
    element.copy_from_slice(&res.into_repr().to_bytes_le());

    // Try the deposit with insufficient fund
    let info = mock_info("depositor", &[Coin::new(1_000_u128, NATIVE_TOKEN_DENOM)]);
    let deposit_msg = DepositMsg {
        commitment: Some(element),
    };

    let err = execute(
        deps.as_mut(),
        mock_env(),
        info,
        ExecuteMsg::Deposit(deposit_msg),
    )
    .unwrap_err();
    assert_eq!(err.to_string(), "Insufficient_funds".to_string());

    // Try the deposit with empty commitment
    let info = mock_info(
        "depositor",
        &[Coin::new(1_000_000_u128, NATIVE_TOKEN_DENOM)],
    );
    let deposit_msg = DepositMsg { commitment: None };

    let err = execute(
        deps.as_mut(),
        mock_env(),
        info,
        ExecuteMsg::Deposit(deposit_msg),
    )
    .unwrap_err();
    assert_eq!(err.to_string(), "Commitment not found".to_string());

    // Try the deposit for success
    let info = mock_info(
        "depositor",
        &[Coin::new(1_000_000_u128, NATIVE_TOKEN_DENOM)],
    );
    let deposit_msg = DepositMsg {
        commitment: Some(element),
    };

    let response = execute(
        deps.as_mut(),
        mock_env(),
        info,
        ExecuteMsg::Deposit(deposit_msg),
    )
    .unwrap();
    assert_eq!(
        response.attributes,
        vec![attr("method", "deposit"), attr("result", "0")]
    );
}

#[test]
fn test_mixer_should_be_able_to_deposit_cw20_token() {
    let mut deps = create_mixer(MixerType::Cw20);

    // Initialize the mixer
    let params = setup_params(Curve::Bn254, 5, 3);
    let poseidon = Poseidon::new(params);
    let res = poseidon.hash_two(&Fr::one(), &Fr::one()).unwrap();
    let mut element: [u8; 32] = [0u8; 32];
    element.copy_from_slice(&res.into_repr().to_bytes_le());

    // Try the deposit for success
    let info = mock_info(CW20_ADDRESS, &[]);
    let deposit_cw20_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: CW20_ADDRESS.to_string(),
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
fn test_mixer_should_work_with_wasm_utils() {
    let (proof_bytes, root_element, nullifier_hash_element, leaf_element) =
        prepare_wasm_utils_zk_circuit(Curve::Bn254, RECIPIENT, RELAYER, FEE, REFUND);
    let mut deps = create_mixer(MixerType::Native);

    // Try the deposit for success
    let info = mock_info(
        "depositor",
        &[Coin::new(1_000_000_u128, NATIVE_TOKEN_DENOM)],
    );
    let deposit_msg = DepositMsg {
        commitment: Some(leaf_element.0),
    };

    let response = execute(
        deps.as_mut(),
        mock_env(),
        info,
        ExecuteMsg::Deposit(deposit_msg).clone(),
    )
    .unwrap();
    assert_eq!(
        response.attributes,
        vec![attr("method", "deposit"), attr("result", "0")]
    );
    let on_chain_root = crate::state::read_root(&deps.storage, 1).unwrap();
    let local_root = root_element.0;
    assert_eq!(on_chain_root, local_root);

    // Should "succeed" to withdraw tokens.
    let withdraw_msg = WithdrawMsg {
        proof_bytes: proof_bytes,
        root: root_element.0,
        nullifier_hash: nullifier_hash_element.0,
        recipient: RECIPIENT.to_string(),
        relayer: RELAYER.to_string(),
        fee: FEE.to_string(),
        refund: REFUND.to_string(),
        cw20_address: None,
    };
    let info = mock_info("withdraw", &[]);
    let response = execute(
        deps.as_mut(),
        mock_env(),
        info,
        ExecuteMsg::Withdraw(withdraw_msg),
    )
    .unwrap();
    assert_eq!(response.attributes, vec![attr("method", "withdraw")]);
}

#[test]
fn test_mixer_fail_when_any_byte_is_changed_in_proof() {
    let (mut proof_bytes, root_element, nullifier_hash_element, leaf_element) =
        prepare_wasm_utils_zk_circuit(Curve::Bn254, RECIPIENT, RELAYER, FEE, REFUND);

    let mut deps = create_mixer(MixerType::Native);

    // Try the deposit for success
    let info = mock_info(
        "depositor",
        &[Coin::new(1_000_000_u128, NATIVE_TOKEN_DENOM)],
    );
    let deposit_msg = DepositMsg {
        commitment: Some(leaf_element.0),
    };

    let response = execute(
        deps.as_mut(),
        mock_env(),
        info,
        ExecuteMsg::Deposit(deposit_msg.clone()),
    )
    .unwrap();
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
        recipient: RECIPIENT.to_string(),
        relayer: RELAYER.to_string(),
        fee: FEE.to_string(),
        refund: REFUND.to_string(),
        cw20_address: None,
    };
    let info = mock_info("withdraw", &[]);
    assert!(
        execute(
            deps.as_mut(),
            mock_env(),
            info,
            ExecuteMsg::Withdraw(withdraw_msg)
        )
        .is_err(),
        "Should fail with wrong proof bytes"
    );
}

#[test]
fn test_mixer_should_withdraw_native_token() {
    let (proof_bytes, root_element, nullifier_hash_element, leaf_element) =
        prepare_zk_circuit(Curve::Bn254, RECIPIENT, RELAYER, FEE, REFUND);

    let mut deps = create_mixer(MixerType::Native);

    // Try the deposit for success
    let info = mock_info(
        "depositor",
        &[Coin::new(1_000_000_u128, NATIVE_TOKEN_DENOM)],
    );
    let deposit_msg = DepositMsg {
        commitment: Some(leaf_element.0),
    };

    let response = execute(
        deps.as_mut(),
        mock_env(),
        info,
        ExecuteMsg::Deposit(deposit_msg.clone()),
    )
    .unwrap();
    assert_eq!(
        response.attributes,
        vec![attr("method", "deposit"), attr("result", "0")]
    );
    let on_chain_root = crate::state::read_root(&deps.storage, 1).unwrap();
    let local_root = root_element.0;
    assert_eq!(on_chain_root, local_root);

    let withdraw_msg = WithdrawMsg {
        proof_bytes: proof_bytes,
        root: root_element.0,
        nullifier_hash: nullifier_hash_element.0,
        recipient: RECIPIENT.to_string(),
        relayer: RELAYER.to_string(),
        fee: FEE.to_string(),
        refund: REFUND.to_string(),
        cw20_address: None,
    };
    let info = mock_info("withdraw", &[]);
    let response = execute(
        deps.as_mut(),
        mock_env(),
        info,
        ExecuteMsg::Withdraw(withdraw_msg),
    )
    .unwrap();
    assert_eq!(response.attributes, vec![attr("method", "withdraw")]);
}

#[test]
fn test_mixer_should_fail_when_invalid_merkle_roots() {
    let (proof_bytes, mut root_element, nullifier_hash_element, leaf_element) =
        prepare_zk_circuit(Curve::Bn254, RECIPIENT, RELAYER, FEE, REFUND);

    let mut deps = create_mixer(MixerType::Native);

    // Try the deposit for success
    let info = mock_info(
        "depositor",
        &[Coin::new(1_000_000_u128, NATIVE_TOKEN_DENOM)],
    );
    let deposit_msg = DepositMsg {
        commitment: Some(leaf_element.0),
    };

    let response = execute(
        deps.as_mut(),
        mock_env(),
        info,
        ExecuteMsg::Deposit(deposit_msg.clone()),
    )
    .unwrap();
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
        recipient: RECIPIENT.to_string(),
        relayer: RELAYER.to_string(),
        fee: FEE.to_string(),
        refund: REFUND.to_string(),
        cw20_address: None,
    };
    let info = mock_info("withdraw", &[]);
    let err = execute(
        deps.as_mut(),
        mock_env(),
        info,
        ExecuteMsg::Withdraw(withdraw_msg),
    )
    .unwrap_err();
    assert_eq!(
        err.to_string(),
        "Generic error: Root is not known".to_string()
    );
}

#[test]
fn test_mixer_should_withdraw_cw20_token() {
    let (proof_bytes, root_element, nullifier_hash_element, leaf_element) =
        prepare_zk_circuit(Curve::Bn254, RECIPIENT, RELAYER, FEE, REFUND);

    let mut deps = create_mixer(MixerType::Cw20);

    // Try the deposit for success
    let info = mock_info(CW20_ADDRESS, &[]);
    let deposit_cw20_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: CW20_ADDRESS.to_string(),
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
        recipient: RECIPIENT.to_string(),
        relayer: RELAYER.to_string(),
        fee: FEE.to_string(),
        refund: REFUND.to_string(),
        cw20_address: Some(CW20_ADDRESS.to_string()),
    };
    let info = mock_info("withdraw", &[]);
    let response = execute(
        deps.as_mut(),
        mock_env(),
        info,
        ExecuteMsg::Withdraw(withdraw_msg),
    )
    .unwrap();
    assert_eq!(response.attributes, vec![attr("method", "withdraw")]);

    let expected_recipient = RECIPIENT.to_string();
    let expected_relayer = RELAYER.to_string();
    let expected_messages: Vec<CosmosMsg> = vec![
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: CW20_ADDRESS.to_string(),
            funds: [].to_vec(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: expected_recipient,
                amount: Uint128::from(1_000_000_u128),
            })
            .unwrap(),
        }),
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: CW20_ADDRESS.to_string(),
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

#[test]
fn test_mixer_should_fail_when_wrong_relayer_input() {
    let (proof_bytes, root_element, nullifier_hash_element, leaf_element) =
        prepare_zk_circuit(Curve::Bn254, RECIPIENT, RELAYER, FEE, REFUND);

    let mut deps = create_mixer(MixerType::Cw20);

    // Try the deposit for success
    let info = mock_info(CW20_ADDRESS, &[]);
    let deposit_cw20_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: CW20_ADDRESS.to_string(),
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

    // Should fail with "Invalid withdraw proof" since "relayer" is changed.
    let withdraw_msg = WithdrawMsg {
        proof_bytes: proof_bytes,
        root: root_element.0,
        nullifier_hash: nullifier_hash_element.0,
        recipient: RECIPIENT.to_string(),
        relayer: "wrong_relayer_address".to_string(),
        fee: FEE.to_string(),
        refund: REFUND.to_string(),
        cw20_address: None,
    };
    let info = mock_info("withdraw", &[]);
    let err = execute(
        deps.as_mut(),
        mock_env(),
        info,
        ExecuteMsg::Withdraw(withdraw_msg),
    )
    .unwrap_err();
    assert_eq!(
        err.to_string(),
        "Generic error: Invalid withdraw proof".to_string()
    );
}

#[test]
fn test_mixer_should_fail_when_fee_submitted_is_changed() {
    let (proof_bytes, root_element, nullifier_hash_element, leaf_element) =
        prepare_zk_circuit(Curve::Bn254, RECIPIENT, RELAYER, FEE, REFUND);

    let mut deps = create_mixer(MixerType::Cw20);

    // Try the deposit for success
    let info = mock_info(CW20_ADDRESS, &[]);
    let deposit_cw20_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: CW20_ADDRESS.to_string(),
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

    // Should fail with "Invalid withdraw proof" since "fee" is changed.
    let withdraw_msg = WithdrawMsg {
        proof_bytes: proof_bytes,
        root: root_element.0,
        nullifier_hash: nullifier_hash_element.0,
        recipient: RECIPIENT.to_string(),
        relayer: "wrong_relayer_address".to_string(),
        fee: "1".to_string(),
        refund: REFUND.to_string(),
        cw20_address: None,
    };
    let info = mock_info("withdraw", &[]);
    let err = execute(
        deps.as_mut(),
        mock_env(),
        info,
        ExecuteMsg::Withdraw(withdraw_msg),
    )
    .unwrap_err();
    assert_eq!(
        err.to_string(),
        "Generic error: Invalid withdraw proof".to_string()
    );
}
