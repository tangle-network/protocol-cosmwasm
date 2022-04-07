use ark_bn254::Fr;
use ark_ff::BigInteger;
use ark_ff::PrimeField;
use ark_std::One;
use arkworks_native_gadgets::poseidon::{FieldHasher, Poseidon};
use arkworks_setups::common::setup_params;
use arkworks_setups::Curve;

use cosmwasm_std::testing::MockApi;
use cosmwasm_std::testing::MockQuerier;
use cosmwasm_std::testing::MockStorage;
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::OwnedDeps;
use cosmwasm_std::{attr, to_binary, CosmosMsg, Uint128, WasmMsg};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};

use protocol_cosmwasm::anchor::{Cw20HookMsg, ExecuteMsg, InstantiateMsg, WithdrawMsg};

use crate::contract::{
    compute_chain_id_type, execute, instantiate, truncate_and_pad, COSMOS_CHAIN_TYPE,
};
#[cfg(test)]
use crate::test_util::Element;

const MAX_EDGES: u32 = 2;
const CHAIN_ID: u64 = 1;
const LEVELS: u32 = 30;
const CW20_ADDRESS: &str = "terra1340t6lqq6jxhm8d6gtz0hzz5jzcszvm27urkn2";
const DEPOSIT_SIZE: u128 = 1_000_000;
const DEPOSITOR: &str = "depositor";

const RECIPIENT: &str = "terra1kejftqzx05y9rv00lw5m76csfmx7lf9se02dz4";
const RELAYER: &str = "terra17cz29kl6z5wj04ledes9jdmn6pgkelffjxglky";
const FEE: u128 = 0;
const REFUND: u128 = 0;

fn create_anchor() -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
    let mut deps = mock_dependencies(&[]);

    let env = mock_env();
    let info = mock_info("anyone", &[]);
    let instantiate_msg = InstantiateMsg {
        max_edges: MAX_EDGES,
        chain_id: CHAIN_ID,
        levels: LEVELS,
        deposit_size: DEPOSIT_SIZE.to_string(),
        cw20_address: CW20_ADDRESS.to_string(),
    };

    // Should pass this "unwrap" if success.
    let _ = instantiate(deps.as_mut(), env, info, instantiate_msg).unwrap();

    deps
}

#[test]
fn test_anchor_proper_initialization() {
    let mut deps = mock_dependencies(&[]);
    let env = mock_env();
    let info = mock_info("anyone", &[]);
    let instantiate_msg = InstantiateMsg {
        max_edges: MAX_EDGES,
        chain_id: CHAIN_ID,
        levels: LEVELS,
        deposit_size: DEPOSIT_SIZE.to_string(),
        cw20_address: CW20_ADDRESS.to_string(),
    };

    // Should pass this "unwrap" if success.
    let response = instantiate(deps.as_mut(), env, info, instantiate_msg).unwrap();

    assert_eq!(
        response.attributes,
        vec![attr("method", "instantiate"), attr("owner", "anyone"),]
    );
}

#[test]
fn test_anchor_should_be_able_to_deposit() {
    let mut deps = create_anchor();

    // Initialize the anchor.
    let params = setup_params(Curve::Bn254, 5, 3);
    let poseidon = Poseidon::new(params);
    let res = poseidon.hash_two(&Fr::one(), &Fr::one()).unwrap();
    let mut element: [u8; 32] = [0u8; 32];
    element.copy_from_slice(&res.into_repr().to_bytes_le());

    // Should "deposit" cw20 tokens with success.
    let info = mock_info(CW20_ADDRESS, &[]);
    let deposit_cw20_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: DEPOSITOR.to_string(),
        amount: Uint128::from(DEPOSIT_SIZE),
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
fn test_anchor_fail_when_any_byte_is_changed_in_proof() {
    let curve = Curve::Bn254;
    let (pk_bytes, _) = crate::test_util::setup_environment(curve);
    let src_chain_id = compute_chain_id_type(1u64, &COSMOS_CHAIN_TYPE);
    let recipient_bytes = RECIPIENT.as_bytes();
    let relayer_bytes = RELAYER.as_bytes();
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
            src_chain_id,
            fee_value,
            refund_value,
        );

    let mut deps = create_anchor();

    // Should "deposit" cw20 tokens with success.
    let info = mock_info(CW20_ADDRESS, &[]);
    let deposit_cw20_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: DEPOSITOR.to_string(),
        amount: Uint128::from(DEPOSIT_SIZE),
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
    let local_root = root_elements[0].0;
    assert_eq!(on_chain_root, local_root);

    // Invalid withdraw proof leads to failure result.
    let mut wrong_proof_bytes = proof_bytes.clone();
    wrong_proof_bytes[0] = 0;

    let mut roots = vec![];
    for i in 0..root_elements.len() {
        roots.push(root_elements[i].0);
    }

    let withdraw_msg = WithdrawMsg {
        proof_bytes: wrong_proof_bytes,
        roots: roots.clone(),
        nullifier_hash: nullifier_hash_element.0,
        recipient: RECIPIENT.to_string(),
        relayer: RELAYER.to_string(),
        fee: FEE.to_string(),
        refund: REFUND.to_string(),
        commitment: commitment_element.0,
        cw20_address: CW20_ADDRESS.to_string(),
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
fn test_anchor_fail_when_invalid_merkle_roots() {
    let curve = Curve::Bn254;
    let (pk_bytes, _) = crate::test_util::setup_environment(curve);
    let src_chain_id = compute_chain_id_type(1u64, &COSMOS_CHAIN_TYPE);
    let recipient_bytes = RECIPIENT.as_bytes();
    let relayer_bytes = RELAYER.as_bytes();
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
            src_chain_id,
            fee_value,
            refund_value,
        );

    let mut deps = create_anchor();

    // Should "deposit" cw20 tokens with success.
    let info = mock_info(CW20_ADDRESS, &[]);
    let deposit_cw20_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: DEPOSITOR.to_string(),
        amount: Uint128::from(DEPOSIT_SIZE),
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
    let local_root = root_elements[0].0;
    assert_eq!(on_chain_root, local_root);

    // Invalid root_element leads to failure.
    let mut wrong_roots = vec![];
    for i in 0..root_elements.len() {
        wrong_roots.push(root_elements[i].0);
    }
    wrong_roots[0][0] = 0;

    let withdraw_msg = WithdrawMsg {
        proof_bytes: proof_bytes.clone(),
        roots: wrong_roots,
        nullifier_hash: nullifier_hash_element.0,
        recipient: RECIPIENT.to_string(),
        relayer: RELAYER.to_string(),
        fee: FEE.to_string(),
        refund: REFUND.to_string(),
        commitment: commitment_element.0,
        cw20_address: CW20_ADDRESS.to_string(),
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
fn test_anchor_works_with_wasm_utils() {
    let curve = Curve::Bn254;
    let (pk_bytes, _) = crate::test_util::setup_environment(curve);
    let src_chain_id = compute_chain_id_type(1u64, &COSMOS_CHAIN_TYPE);
    let recipient_bytes = RECIPIENT.as_bytes();
    let relayer_bytes = RELAYER.as_bytes();
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
            src_chain_id,
            fee_value,
            refund_value,
        );

    let mut deps = create_anchor();

    // Should "deposit" cw20 tokens with success.
    let info = mock_info(CW20_ADDRESS, &[]);
    let deposit_cw20_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: DEPOSITOR.to_string(),
        amount: Uint128::from(DEPOSIT_SIZE),
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
    let local_root = root_elements[0].0;
    assert_eq!(on_chain_root, local_root);

    let mut roots = vec![];
    for i in 0..root_elements.len() {
        roots.push(root_elements[i].0);
    }

    // Should "withdraw" cw20 tokens with success.
    let withdraw_msg = WithdrawMsg {
        proof_bytes: proof_bytes,
        roots: roots,
        nullifier_hash: nullifier_hash_element.0,
        recipient: RECIPIENT.to_string(),
        relayer: RELAYER.to_string(),
        fee: FEE.to_string(),
        refund: REFUND.to_string(),
        commitment: commitment_element.0,
        cw20_address: CW20_ADDRESS.to_string(),
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
    let expected_messages: Vec<CosmosMsg> = vec![CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: CW20_ADDRESS.to_string(),
        funds: [].to_vec(),
        msg: to_binary(&Cw20ExecuteMsg::Transfer {
            recipient: expected_recipient,
            amount: Uint128::from(DEPOSIT_SIZE),
        })
        .unwrap(),
    })];
    assert_eq!(response.messages.len(), expected_messages.len());
}

#[test]
fn test_anchor_works() {
    let curve = Curve::Bn254;
    let (pk_bytes, _) = crate::test_util::setup_environment(curve);
    let recipient_bytes = RECIPIENT.as_bytes();
    let relayer_bytes = RELAYER.as_bytes();
    let fee_value = 0;
    let refund_value = 0;
    let src_chain_id = compute_chain_id_type(1, &COSMOS_CHAIN_TYPE);
    let commitment_bytes = vec![0u8; 32];
    let commitment_element = Element::from_bytes(&commitment_bytes);

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

    let mut deps = create_anchor();

    // Should "deposit" cw20 tokens with success.
    let info = mock_info(CW20_ADDRESS, &[]);
    let deposit_cw20_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: DEPOSITOR.to_string(),
        amount: Uint128::from(DEPOSIT_SIZE),
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
    let local_root = root_elements[0].0;
    assert_eq!(on_chain_root, local_root);

    // Should "withdraw" cw20 tokens with success.
    let mut roots = vec![];
    for elem in root_elements {
        roots.push(elem.0);
    }
    let withdraw_msg = WithdrawMsg {
        proof_bytes: proof_bytes,
        roots: roots,
        nullifier_hash: nullifier_hash_element.0,
        recipient: RECIPIENT.to_string(),
        relayer: RELAYER.to_string(),
        fee: FEE.to_string(),
        refund: REFUND.to_string(),
        commitment: commitment_element.0,
        cw20_address: CW20_ADDRESS.to_string(),
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
    let expected_messages: Vec<CosmosMsg> = vec![CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: CW20_ADDRESS.to_string(),
        funds: [].to_vec(),
        msg: to_binary(&Cw20ExecuteMsg::Transfer {
            recipient: expected_recipient,
            amount: Uint128::from(DEPOSIT_SIZE),
        })
        .unwrap(),
    })];
    assert_eq!(response.messages.len(), expected_messages.len());
}

#[test]
fn test_anchor_fail_when_relayer_is_diff_from_that_in_proof_generation() {
    let curve = Curve::Bn254;
    let (pk_bytes, _) = crate::test_util::setup_environment(curve);
    let recipient_bytes = RECIPIENT.as_bytes();
    let relayer_bytes = RELAYER.as_bytes();
    let fee_value = 0;
    let refund_value = 0;
    let src_chain_id = compute_chain_id_type(1, &COSMOS_CHAIN_TYPE);
    let commitment_bytes = vec![0u8; 32];
    let commitment_element = Element::from_bytes(&commitment_bytes);

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

    let mut deps = create_anchor();

    // Should "deposit" cw20 tokens with success.
    let info = mock_info(CW20_ADDRESS, &[]);
    let deposit_cw20_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: DEPOSITOR.to_string(),
        amount: Uint128::from(DEPOSIT_SIZE),
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
    let local_root = root_elements[0].0;
    assert_eq!(on_chain_root, local_root);

    // Should fail with "wrong relayer" error.
    let mut roots = vec![];
    for elem in root_elements {
        roots.push(elem.0);
    }
    let wrong_relayer_bytes = [0u8; 32];
    let withdraw_msg = WithdrawMsg {
        proof_bytes: proof_bytes,
        roots: roots,
        nullifier_hash: nullifier_hash_element.0,
        recipient: RECIPIENT.to_string(),
        relayer: hex::encode(wrong_relayer_bytes.to_vec()),
        fee: FEE.to_string(),
        refund: REFUND.to_string(),
        commitment: commitment_element.0,
        cw20_address: CW20_ADDRESS.to_string(),
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
fn test_anchor_fail_when_fee_submitted_is_changed() {
    let curve = Curve::Bn254;
    let (pk_bytes, _) = crate::test_util::setup_environment(curve);
    let recipient_bytes = RECIPIENT.as_bytes();
    let relayer_bytes = RELAYER.as_bytes();
    let fee_value = 0;
    let refund_value = 0;
    let src_chain_id = compute_chain_id_type(1, &COSMOS_CHAIN_TYPE);
    let commitment_bytes = vec![0u8; 32];
    let commitment_element = Element::from_bytes(&commitment_bytes);

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

    let mut deps = create_anchor();

    // Should "deposit" cw20 tokens with success.
    let info = mock_info(CW20_ADDRESS, &[]);
    let deposit_cw20_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: DEPOSITOR.to_string(),
        amount: Uint128::from(DEPOSIT_SIZE),
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
    let local_root = root_elements[0].0;
    assert_eq!(on_chain_root, local_root);

    // Should fail with "Invalid withdraw proof" since "fee" changed.
    let mut roots = vec![];
    for elem in root_elements {
        roots.push(elem.0);
    }
    let changed_fee_value = 1u128;
    let withdraw_msg = WithdrawMsg {
        proof_bytes: proof_bytes,
        roots: roots,
        nullifier_hash: nullifier_hash_element.0,
        recipient: RECIPIENT.to_string(),
        relayer: RELAYER.to_string(),
        fee: changed_fee_value.to_string(),
        refund: REFUND.to_string(),
        commitment: commitment_element.0,
        cw20_address: CW20_ADDRESS.to_string(),
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
