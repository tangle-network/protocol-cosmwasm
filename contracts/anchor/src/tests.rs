use ark_bn254::Fr;
use ark_crypto_primitives::CRH as CRHTrait;
use ark_ff::PrimeField;
use ark_ff::{BigInteger, Field};
use ark_std::One;
use arkworks_gadgets::poseidon::CRH;
use arkworks_utils::utils::bn254_x5_5::get_poseidon_bn254_x5_5;
use arkworks_utils::utils::common::Curve;

use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{attr, coins, from_binary, to_binary, CosmosMsg, Uint128, WasmMsg};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};

use protocol_cosmwasm::anchor::{
    Cw20HookMsg, ExecuteMsg, InfoResponse, InstantiateMsg, QueryMsg, WithdrawMsg,
};

use crate::contract::{
    compute_chain_id_type, execute, instantiate, query, truncate_and_pad, COSMOS_CHAIN_TYPE,
};
#[cfg(test)]
use crate::test_util::Element;

type PoseidonCRH5 = CRH<ark_bn254::Fr>;

#[test]
fn test_anchor_proper_initialization() {
    let cw20_address = "terra1fex9f78reuwhfsnc8sun6mz8rl9zwqh03fhwf3".to_string();
    let mut deps = mock_dependencies(&[]);
    let env = mock_env();
    let info = mock_info("anyone", &[]);
    let instantiate_msg = InstantiateMsg {
        max_edges: 0,
        chain_id: 1,
        levels: 0,
        deposit_size: Uint128::from(1_000_000_u128),
        cw20_address: cw20_address.clone(),
    };

    // Should pass this "unwrap" if success.
    let response = instantiate(deps.as_mut(), env, info, instantiate_msg).unwrap();

    assert_eq!(
        response.attributes,
        vec![attr("method", "instantiate"), attr("owner", "anyone"),]
    );

    let query = query(deps.as_ref(), mock_env(), QueryMsg::GetCw20Address {}).unwrap();
    let info: InfoResponse = from_binary(&query).unwrap();
    assert_eq!(info.cw20_address, cw20_address);
}

#[test]
fn test_anchor_should_be_able_to_deposit() {
    let cw20_address = "terra1fex9f78reuwhfsnc8sun6mz8rl9zwqh03fhwf3".to_string();

    let mut deps = mock_dependencies(&coins(2, "token"));

    // Initialize the contract
    let env = mock_env();
    let info = mock_info("anyone", &[]);
    let instantiate_msg = InstantiateMsg {
        max_edges: 2,
        chain_id: 1,
        levels: 30,
        deposit_size: Uint128::from(1_000_000_u128),
        cw20_address: cw20_address.clone(),
    };

    let _ = instantiate(deps.as_mut(), env, info, instantiate_msg).unwrap();

    // Initialize the anchor.
    let params = get_poseidon_bn254_x5_5();
    let left_input = Fr::one().into_repr().to_bytes_le();
    let right_input = Fr::one().double().into_repr().to_bytes_le();
    let mut input = Vec::new();
    input.extend_from_slice(&left_input);
    input.extend_from_slice(&right_input);
    let res = <PoseidonCRH5 as CRHTrait>::evaluate(&params, &input).unwrap();
    let mut element: [u8; 32] = [0u8; 32];
    element.copy_from_slice(&res.into_repr().to_bytes_le());

    // Should "deposit" cw20 tokens with success.
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
fn test_anchor_fail_when_any_byte_is_changed_in_proof() {
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

    let cw20_address = "terra1fex9f78reuwhfsnc8sun6mz8rl9zwqh03fhwf3".to_string();
    let mut deps = mock_dependencies(&coins(2, "token"));

    // Initialize the contract
    let env = mock_env();
    let info = mock_info("anyone", &[]);
    let instantiate_msg = InstantiateMsg {
        max_edges: 2,
        chain_id: 1,
        levels: 30,
        deposit_size: Uint128::from(1_000_000_u128),
        cw20_address: cw20_address.clone(),
    };

    let _ = instantiate(deps.as_mut(), env, info, instantiate_msg).unwrap();

    // Should "deposit" cw20 tokens with success.
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
        recipient: hex::encode(recipient_bytes.to_vec()),
        relayer: hex::encode(relayer_bytes.to_vec()),
        fee: cosmwasm_std::Uint256::from(fee_value),
        refund: cosmwasm_std::Uint256::from(refund_value),
        commitment: commitment_element.0,
        cw20_address: cw20_address.clone(),
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

    let cw20_address = "terra1fex9f78reuwhfsnc8sun6mz8rl9zwqh03fhwf3".to_string();
    let mut deps = mock_dependencies(&coins(2, "token"));

    // Initialize the contract
    let env = mock_env();
    let info = mock_info("anyone", &[]);
    let instantiate_msg = InstantiateMsg {
        max_edges: 2,
        chain_id: 1,
        levels: 30,
        deposit_size: Uint128::from(1_000_000_u128),
        cw20_address: cw20_address.clone(),
    };

    let _ = instantiate(deps.as_mut(), env, info, instantiate_msg).unwrap();

    // Should "deposit" cw20 tokens with success.
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
        recipient: hex::encode(recipient_bytes.to_vec()),
        relayer: hex::encode(relayer_bytes.to_vec()),
        fee: cosmwasm_std::Uint256::from(fee_value),
        refund: cosmwasm_std::Uint256::from(refund_value),
        commitment: commitment_element.0,
        cw20_address: cw20_address.clone(),
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

    let cw20_address = "terra1fex9f78reuwhfsnc8sun6mz8rl9zwqh03fhwf3".to_string();
    let mut deps = mock_dependencies(&coins(2, "token"));

    // Initialize the contract
    let env = mock_env();
    let info = mock_info("anyone", &[]);
    let instantiate_msg = InstantiateMsg {
        max_edges: 2,
        chain_id: 1,
        levels: 30,
        deposit_size: Uint128::from(1_000_000_u128),
        cw20_address: cw20_address.clone(),
    };

    let _ = instantiate(deps.as_mut(), env, info, instantiate_msg).unwrap();

    // Should "deposit" cw20 tokens with success.
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
        recipient: hex::encode(recipient_bytes.to_vec()),
        relayer: hex::encode(relayer_bytes.to_vec()),
        fee: cosmwasm_std::Uint256::from(fee_value),
        refund: cosmwasm_std::Uint256::from(refund_value),
        commitment: commitment_element.0,
        cw20_address: cw20_address.clone(),
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

#[test]
fn test_anchor_works() {
    let curve = Curve::Bn254;
    let (pk_bytes, _) = crate::test_util::setup_environment(curve);
    let recipient_bytes = [1u8; 32];
    let relayer_bytes = [2u8; 32];
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

    let cw20_address = "terra1fex9f78reuwhfsnc8sun6mz8rl9zwqh03fhwf3".to_string();
    let mut deps = mock_dependencies(&coins(2, "token"));

    // Initialize the contract
    let env = mock_env();
    let info = mock_info("anyone", &[]);
    let instantiate_msg = InstantiateMsg {
        max_edges: 2,
        chain_id: 1,
        levels: 30,
        deposit_size: Uint128::from(1_000_000_u128),
        cw20_address: cw20_address.clone(),
    };

    let _ = instantiate(deps.as_mut(), env, info, instantiate_msg).unwrap();

    // Should "deposit" cw20 tokens with success.
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
        recipient: hex::encode(recipient_bytes.to_vec()),
        relayer: hex::encode(relayer_bytes.to_vec()),
        fee: cosmwasm_std::Uint256::from(fee_value),
        refund: cosmwasm_std::Uint256::from(refund_value),
        commitment: commitment_element.0,
        cw20_address: cw20_address.clone(),
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

#[test]
fn test_anchor_fail_when_relayer_is_diff_from_that_in_proof_generation() {
    let curve = Curve::Bn254;
    let (pk_bytes, _) = crate::test_util::setup_environment(curve);
    let recipient_bytes = [1u8; 32];
    let relayer_bytes = [2u8; 32];
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

    let cw20_address = "terra1fex9f78reuwhfsnc8sun6mz8rl9zwqh03fhwf3".to_string();
    let mut deps = mock_dependencies(&coins(2, "token"));

    // Initialize the contract
    let env = mock_env();
    let info = mock_info("anyone", &[]);
    let instantiate_msg = InstantiateMsg {
        max_edges: 2,
        chain_id: 1,
        levels: 30,
        deposit_size: Uint128::from(1_000_000_u128),
        cw20_address: cw20_address.clone(),
    };

    let _ = instantiate(deps.as_mut(), env, info, instantiate_msg).unwrap();

    // Should "deposit" cw20 tokens with success.
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
        recipient: hex::encode(recipient_bytes.to_vec()),
        relayer: hex::encode(wrong_relayer_bytes.to_vec()),
        fee: cosmwasm_std::Uint256::from(fee_value),
        refund: cosmwasm_std::Uint256::from(refund_value),
        commitment: commitment_element.0,
        cw20_address: cw20_address.clone(),
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
    let recipient_bytes = [1u8; 32];
    let relayer_bytes = [2u8; 32];
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

    let cw20_address = "terra1fex9f78reuwhfsnc8sun6mz8rl9zwqh03fhwf3".to_string();
    let mut deps = mock_dependencies(&coins(2, "token"));

    // Initialize the contract
    let env = mock_env();
    let info = mock_info("anyone", &[]);
    let instantiate_msg = InstantiateMsg {
        max_edges: 2,
        chain_id: 1,
        levels: 30,
        deposit_size: Uint128::from(1_000_000_u128),
        cw20_address: cw20_address.clone(),
    };

    let _ = instantiate(deps.as_mut(), env, info, instantiate_msg).unwrap();

    // Should "deposit" cw20 tokens with success.
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
        recipient: hex::encode(recipient_bytes.to_vec()),
        relayer: hex::encode(relayer_bytes.to_vec()),
        fee: cosmwasm_std::Uint256::from(changed_fee_value),
        refund: cosmwasm_std::Uint256::from(refund_value),
        commitment: commitment_element.0,
        cw20_address: cw20_address.clone(),
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
