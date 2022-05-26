use ark_ff::BigInteger;
use ark_ff::PrimeField;
use arkworks_setups::Curve;
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info, MockApi, MockStorage};
use cosmwasm_std::{attr, coins, to_binary, OwnedDeps, Uint128};
use cw20::Cw20ReceiveMsg;
use protocol_cosmwasm::error::ContractError;
use sp_core::hashing::keccak_256;

use crate::contract::{execute, instantiate};
use protocol_cosmwasm::utils::compute_chain_id_type;
use protocol_cosmwasm::vanchor::{
    Cw20HookMsg, ExecuteMsg, ExtData, InstantiateMsg, ProofData, UpdateConfigMsg,
};
use protocol_cosmwasm::zeroes::zeroes;

const CHAIN_TYPE: [u8; 2] = [4, 0]; // 0x0400
const TEST_CHAIN_ID: u64 = 1;

const MAX_EDGES: u32 = 2;
const LEVELS: u32 = 30;
const MAX_DEPOSIT_AMT: u128 = 400;
const MIN_WITHDRAW_AMT: u128 = 0;
const MAX_EXT_AMT: u128 = 200;
const MAX_FEE: u128 = 100;

const CW20_ADDRESS: &str = "terra1340t6lqq6jxhm8d6gtz0hzz5jzcszvm27urkn2";
const TRANSACTOR: &str = "juno1yq0azfkky8aqq4kvzdawrs7tm3rmpl8xs6vcx2";
const RECIPIENT: &str = "juno16e3t7td2wu0wmggnxa3xnyu5whljyed69ptvkp";
const RELAYER: &str = "juno16g2rahf5846rxzp3fwlswy08fz8ccuwk03k57y";
const HANDLER: &str = "terra1fex9f78reuwhfsnc8sun6mz8rl9zwqh03fhwf3";

fn element_encoder(v: &[u8]) -> [u8; 32] {
    let mut output = [0u8; 32];
    output.iter_mut().zip(v).for_each(|(b1, b2)| *b1 = *b2);
    output
}

fn create_vanchor() -> OwnedDeps<MockStorage, MockApi, crate::mock_querier::WasmMockQuerier> {
    let mut deps = crate::mock_querier::mock_dependencies(&[]);

    let msg = InstantiateMsg {
        chain_id: TEST_CHAIN_ID,
        max_edges: MAX_EDGES,
        levels: LEVELS,
        max_deposit_amt: Uint128::from(MAX_DEPOSIT_AMT),
        min_withdraw_amt: Uint128::from(MIN_WITHDRAW_AMT),
        max_ext_amt: Uint128::from(MAX_EXT_AMT),
        max_fee: Uint128::from(MAX_FEE),
        tokenwrapper_addr: CW20_ADDRESS.to_string(),
        handler: HANDLER.to_string(),
    };
    let info = mock_info("creator", &[]);

    // we can just call .unwrap() to assert this was a success
    let _ = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    deps
}

fn hash_ext_data(ext_data: ExtData, ext_amount: i128, fee: u128) -> [u8; 32] {
    let mut ext_data_args = Vec::new();
    let recipient_bytes = element_encoder(ext_data.recipient.as_bytes());
    let relayer_bytes = element_encoder(ext_data.relayer.as_bytes());
    let fee_bytes = element_encoder(&fee.to_le_bytes());
    let ext_amt_bytes = element_encoder(&ext_amount.to_le_bytes());
    ext_data_args.extend_from_slice(&recipient_bytes);
    ext_data_args.extend_from_slice(&relayer_bytes);
    ext_data_args.extend_from_slice(&ext_amt_bytes);
    ext_data_args.extend_from_slice(&fee_bytes);
    ext_data_args.extend_from_slice(&ext_data.encrypted_output1);
    ext_data_args.extend_from_slice(&ext_data.encrypted_output2);

    keccak_256(&ext_data_args)
}

#[test]
fn test_vanchor_proper_initialization() {
    let mut deps = mock_dependencies();

    let msg = InstantiateMsg {
        chain_id: TEST_CHAIN_ID,
        max_edges: MAX_EDGES,
        levels: LEVELS,
        max_deposit_amt: Uint128::from(MAX_DEPOSIT_AMT),
        min_withdraw_amt: Uint128::from(MIN_WITHDRAW_AMT),
        max_ext_amt: Uint128::from(MAX_EXT_AMT),
        max_fee: Uint128::from(MAX_FEE),
        tokenwrapper_addr: CW20_ADDRESS.to_string(),
        handler: HANDLER.to_string(),
    };
    let info = mock_info("creator", &[]);

    // we can just call .unwrap() to assert this was a success
    let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());
}

#[test]
fn test_vanchor_update_config() {
    let mut deps = create_vanchor();

    // Fail to update the config with "unauthorized" error.
    let update_config_msg = UpdateConfigMsg {
        max_ext_amt: Some(Uint128::from(1u128)),
        max_fee: Some(Uint128::from(1u128)),
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
        max_ext_amt: Some(Uint128::from(1u128)),
        max_fee: Some(Uint128::from(1u128)),
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
fn test_vanchor_should_complete_2x2_transaction_with_deposit_cw20() {
    // Instantiate the "vanchor" contract.
    let mut deps = create_vanchor();

    // Initialize the vanchor
    let (pk_bytes, _) = crate::test_util::setup_environment_2_2_2(Curve::Bn254);
    let ext_amount = 10_i128;
    let fee = 0_u128;

    let public_amount = 10_i128;

    let chain_type = [4, 0];
    let chain_id = compute_chain_id_type(1, &chain_type);
    let in_chain_ids = [chain_id; 2];
    let in_amounts = [0, 0];
    let in_indices = [0, 1];
    let out_chain_ids = [chain_id; 2];
    let out_amounts = [10, 0];

    let in_utxos = crate::test_util::setup_utxos_2_2_2(in_chain_ids, in_amounts, Some(in_indices));
    // We are adding indices to out utxos, since they will be used as an input utxos in next transaction
    let out_utxos =
        crate::test_util::setup_utxos_2_2_2(out_chain_ids, out_amounts, Some(in_indices));

    let output1 = out_utxos[0].commitment.into_repr().to_bytes_le();
    let output2 = out_utxos[1].commitment.into_repr().to_bytes_le();

    let ext_data = ExtData {
        recipient: RECIPIENT.to_string(),
        relayer: RELAYER.to_string(),
        ext_amount: ext_amount.to_string(),
        fee: Uint128::from(fee),
        encrypted_output1: element_encoder(&output1),
        encrypted_output2: element_encoder(&output2),
    };

    let ext_data_hash = hash_ext_data(ext_data.clone(), ext_amount, fee);

    let custom_roots = Some([zeroes(LEVELS), zeroes(LEVELS)].map(|x| x.to_vec()));
    let (proof, public_inputs) = crate::test_util::setup_zk_circuit_2_2_2(
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
        crate::test_util::deconstruct_public_inputs_el_2_2_2(&public_inputs);

    // Constructing proof data
    let root_set = root_set.into_iter().map(|v| v.0).collect();
    let nullifiers = nullifiers.into_iter().map(|v| v.0).collect();
    let commitments = commitments.into_iter().map(|v| v.0).collect();
    let proof_data = ProofData::new(
        proof,
        public_amount.0,
        root_set,
        nullifiers,
        commitments,
        ext_data_hash.0,
    );

    // Should "transact" with success.
    let info = mock_info(CW20_ADDRESS, &[]);
    let deposit_cw20_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: TRANSACTOR.to_string(),
        amount: Uint128::from(10_u128),
        msg: to_binary(&Cw20HookMsg::TransactDeposit {
            proof_data: proof_data,
            ext_data: ext_data,
        })
        .unwrap(),
    });

    let response = execute(deps.as_mut(), mock_env(), info, deposit_cw20_msg).unwrap();
    assert_eq!(
        response.attributes,
        vec![attr("method", "transact_deposit"), attr("ext_amt", "10"),]
    );
}

#[test]
fn test_vanchor_should_complete_2x2_transaction_with_withdraw_cw20() {
    // Instantiate the "vanchor" contract.
    let mut deps = create_vanchor();

    // Initialize the vanchor
    let (pk_bytes, _) = crate::test_util::setup_environment_2_2_2(Curve::Bn254);
    let ext_amount = 10_i128;
    let fee = 0_u128;

    let public_amount = 10_i128;

    let chain_type = [4, 0];
    let chain_id = compute_chain_id_type(1, &chain_type);
    let in_chain_ids = [chain_id; 2];
    let in_amounts = [0, 0];
    let in_indices = [0, 1];
    let out_chain_ids = [chain_id; 2];
    let out_amounts = [10, 0];

    let in_utxos = crate::test_util::setup_utxos_2_2_2(in_chain_ids, in_amounts, Some(in_indices));
    // We are adding indices to out utxos, since they will be used as an input utxos in next transaction
    let out_utxos =
        crate::test_util::setup_utxos_2_2_2(out_chain_ids, out_amounts, Some(in_indices));

    let output1 = out_utxos[0].commitment.into_repr().to_bytes_le();
    let output2 = out_utxos[1].commitment.into_repr().to_bytes_le();

    let ext_data = ExtData {
        recipient: RECIPIENT.to_string(),
        relayer: RELAYER.to_string(),
        ext_amount: ext_amount.to_string(),
        fee: Uint128::from(fee),
        encrypted_output1: element_encoder(&output1),
        encrypted_output2: element_encoder(&output2),
    };

    let ext_data_hash = hash_ext_data(ext_data.clone(), ext_amount, fee);

    let custom_roots = Some([zeroes(LEVELS), zeroes(LEVELS)].map(|x| x.to_vec()));
    let (proof, public_inputs) = crate::test_util::setup_zk_circuit_2_2_2(
        public_amount,
        chain_id,
        ext_data_hash.to_vec(),
        in_utxos,
        out_utxos.clone(),
        custom_roots,
        pk_bytes,
    );

    // Deconstructing public inputs
    let (_chain_id, public_amount, root_set, nullifiers, commitments, ext_data_hash) =
        crate::test_util::deconstruct_public_inputs_el_2_2_2(&public_inputs);

    // Constructing proof data
    let root_set = root_set.into_iter().map(|v| v.0).collect();
    let nullifiers = nullifiers.into_iter().map(|v| v.0).collect();
    let commitments = commitments.into_iter().map(|v| v.0).collect();
    let proof_data = ProofData::new(
        proof,
        public_amount.0,
        root_set,
        nullifiers,
        commitments,
        ext_data_hash.0,
    );

    // Should "transact" with success.
    let info = mock_info(CW20_ADDRESS, &[]);
    let deposit_cw20_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: TRANSACTOR.to_string(),
        amount: Uint128::from(10_u128),
        msg: to_binary(&Cw20HookMsg::TransactDeposit {
            proof_data: proof_data,
            ext_data: ext_data,
        })
        .unwrap(),
    });

    // Deposit "10" cw20 tokens.
    let _ = execute(deps.as_mut(), mock_env(), info, deposit_cw20_msg).unwrap();

    // Prepare the "withdraw" data.
    let (pk_bytes, _) = crate::test_util::setup_environment_2_2_2(Curve::Bn254);

    let ext_amount = -5_i128;
    let fee = 2_u128;

    let public_amount = -7_i128;

    let chain_id = compute_chain_id_type(TEST_CHAIN_ID, &CHAIN_TYPE);
    let out_chain_ids = [TEST_CHAIN_ID; 2];
    // After withdrawing -7
    let out_amounts = [1, 2];

    // "in_utxos" become the "out_utxos" of last transact.
    let in_utxos = out_utxos;
    let out_utxos = crate::test_util::setup_utxos_2_2_2(out_chain_ids, out_amounts, None);

    let output1 = out_utxos[0].commitment.into_repr().to_bytes_le();
    let output2 = out_utxos[1].commitment.into_repr().to_bytes_le();
    let ext_data = ExtData {
        recipient: RECIPIENT.to_string(),
        relayer: RELAYER.to_string(),
        ext_amount: ext_amount.to_string(),
        fee: Uint128::from(fee),
        encrypted_output1: element_encoder(&output1),
        encrypted_output2: element_encoder(&output2),
    };

    let ext_data_hash = hash_ext_data(ext_data.clone(), ext_amount, fee);

    let (proof, public_inputs) = crate::test_util::setup_zk_circuit_2_2_2(
        public_amount,
        chain_id,
        ext_data_hash.to_vec(),
        in_utxos,
        out_utxos,
        None,
        pk_bytes,
    );

    // Deconstructing public inputs
    let (_chain_id, public_amount, root_set, nullifiers, commitments, ext_data_hash) =
        crate::test_util::deconstruct_public_inputs_el_2_2_2(&public_inputs);

    // Constructing proof data
    let root_set = root_set.into_iter().map(|v| v.0).collect();
    let nullifiers = nullifiers.into_iter().map(|v| v.0).collect();
    let commitments = commitments.into_iter().map(|v| v.0).collect();

    let proof_data = ProofData::new(
        proof,
        public_amount.0,
        root_set,
        nullifiers,
        commitments,
        ext_data_hash.0,
    );

    // Should "transact" with success.
    let info = mock_info(CW20_ADDRESS, &[]);
    let withdraw_cw20_msg = ExecuteMsg::TransactWithdraw {
        proof_data: proof_data,
        ext_data: ext_data,
    };

    // Withdraw "7" cw20 tokens.
    let response = execute(deps.as_mut(), mock_env(), info, withdraw_cw20_msg).unwrap();
    assert_eq!(
        response.attributes,
        vec![attr("method", "transact_withdraw"), attr("ext_amt", "-5"),]
    );
}

#[test]
fn test_vanchor_should_not_complete_transaction_if_ext_data_is_invalid() {
    // Instantiate the "vanchor" contract.
    let mut deps = create_vanchor();

    // Initialize the vanchor
    let (pk_bytes, _) = crate::test_util::setup_environment_2_2_2(Curve::Bn254);
    let ext_amount = 10_i128;
    let fee = 0_u128;

    let public_amount = 10_i128;

    let chain_type = [4, 0];
    let chain_id = compute_chain_id_type(1, &chain_type);
    let in_chain_ids = [chain_id; 2];
    let in_amounts = [0, 0];
    let in_indices = [0, 1];
    let out_chain_ids = [chain_id; 2];
    let out_amounts = [10, 0];

    let in_utxos = crate::test_util::setup_utxos_2_2_2(in_chain_ids, in_amounts, Some(in_indices));
    // We are adding indices to out utxos, since they will be used as an input utxos in next transaction
    let out_utxos =
        crate::test_util::setup_utxos_2_2_2(out_chain_ids, out_amounts, Some(in_indices));

    let output1 = out_utxos[0].commitment.into_repr().to_bytes_le();
    let output2 = out_utxos[1].commitment.into_repr().to_bytes_le();

    let ext_data = ExtData {
        recipient: RECIPIENT.to_string(),
        relayer: RELAYER.to_string(),
        ext_amount: ext_amount.to_string(),
        fee: Uint128::from(fee),
        encrypted_output1: element_encoder(&output1),
        encrypted_output2: element_encoder(&output2),
    };

    let ext_data_hash = hash_ext_data(ext_data.clone(), ext_amount, fee);

    let custom_roots = Some([zeroes(LEVELS), zeroes(LEVELS)].map(|x| x.to_vec()));
    let (proof, public_inputs) = crate::test_util::setup_zk_circuit_2_2_2(
        public_amount,
        chain_id,
        ext_data_hash.to_vec(),
        in_utxos,
        out_utxos.clone(),
        custom_roots,
        pk_bytes,
    );

    // Deconstructing public inputs
    let (_chain_id, public_amount, root_set, nullifiers, commitments, ext_data_hash) =
        crate::test_util::deconstruct_public_inputs_el_2_2_2(&public_inputs);

    // Constructing proof data
    let root_set = root_set.into_iter().map(|v| v.0).collect();
    let nullifiers = nullifiers.into_iter().map(|v| v.0).collect();
    let commitments = commitments.into_iter().map(|v| v.0).collect();
    let proof_data = ProofData::new(
        proof,
        public_amount.0,
        root_set,
        nullifiers,
        commitments,
        ext_data_hash.0,
    );

    // Should "transact" with success.
    let info = mock_info(CW20_ADDRESS, &[]);
    let deposit_cw20_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: TRANSACTOR.to_string(),
        amount: Uint128::from(10_u128),
        msg: to_binary(&Cw20HookMsg::TransactDeposit {
            proof_data: proof_data,
            ext_data: ext_data,
        })
        .unwrap(),
    });

    // Deposit "10" cw20 tokens.
    let _ = execute(deps.as_mut(), mock_env(), info, deposit_cw20_msg).unwrap();

    // Prepare the "withdraw" data.
    let (pk_bytes, _) = crate::test_util::setup_environment_2_2_2(Curve::Bn254);

    let ext_amount = -5_i128;
    let fee = 2_u128;

    let public_amount = -7_i128;

    let chain_id = compute_chain_id_type(TEST_CHAIN_ID, &CHAIN_TYPE);
    let out_chain_ids = [TEST_CHAIN_ID; 2];
    // After withdrawing -7
    let out_amounts = [1, 2];

    // "in_utxos" become the "out_utxos" of last transact.
    let in_utxos = out_utxos;
    let out_utxos = crate::test_util::setup_utxos_2_2_2(out_chain_ids, out_amounts, None);

    let output1 = out_utxos[0].commitment.into_repr().to_bytes_le();
    let output2 = out_utxos[1].commitment.into_repr().to_bytes_le();
    let ext_data = ExtData {
        recipient: RECIPIENT.to_string(),
        relayer: RELAYER.to_string(),
        ext_amount: ext_amount.to_string(),
        fee: Uint128::from(fee),
        encrypted_output1: element_encoder(&output1),
        encrypted_output2: element_encoder(&output2),
    };

    let ext_data_hash = hash_ext_data(ext_data.clone(), ext_amount, fee);

    let (proof, public_inputs) = crate::test_util::setup_zk_circuit_2_2_2(
        public_amount,
        chain_id,
        ext_data_hash.to_vec(),
        in_utxos,
        out_utxos,
        None,
        pk_bytes,
    );

    // Deconstructing public inputs
    let (_chain_id, public_amount, root_set, nullifiers, commitments, ext_data_hash) =
        crate::test_util::deconstruct_public_inputs_el_2_2_2(&public_inputs);

    // Invalid ext data.
    let ext_data = ExtData {
        recipient: RECIPIENT.to_string(),
        relayer: RELAYER.to_string(),
        ext_amount: ext_amount.to_string(),
        fee: Uint128::from(fee),
        encrypted_output1: element_encoder(&output1),
        encrypted_output2: [0u8; 32],
    };

    // Constructing proof data
    let root_set = root_set.into_iter().map(|v| v.0).collect();
    let nullifiers = nullifiers.into_iter().map(|v| v.0).collect();
    let commitments = commitments.into_iter().map(|v| v.0).collect();

    let proof_data = ProofData::new(
        proof,
        public_amount.0,
        root_set,
        nullifiers,
        commitments,
        ext_data_hash.0,
    );

    // Should fail with "invalid ext data".
    let info = mock_info(CW20_ADDRESS, &[]);
    let withdraw_cw20_msg = ExecuteMsg::TransactWithdraw {
        proof_data: proof_data,
        ext_data: ext_data,
    };

    let err = execute(deps.as_mut(), mock_env(), info, withdraw_cw20_msg).unwrap_err();
    assert_eq!(err.to_string(), "Invalid ext data".to_string());
}

#[test]
fn test_vanchor_should_not_complete_withdraw_if_out_amount_sum_is_too_big() {
    // Instantiate the "vanchor" contract.
    let mut deps = create_vanchor();

    // Initialize the vanchor
    let (pk_bytes, _) = crate::test_util::setup_environment_2_2_2(Curve::Bn254);
    let ext_amount = 10_i128;
    let fee = 0_u128;

    let public_amount = 10_i128;

    let chain_type = [4, 0];
    let chain_id = compute_chain_id_type(1, &chain_type);
    let in_chain_ids = [chain_id; 2];
    let in_amounts = [0, 0];
    let in_indices = [0, 1];
    let out_chain_ids = [chain_id; 2];
    let out_amounts = [10, 0];

    let in_utxos = crate::test_util::setup_utxos_2_2_2(in_chain_ids, in_amounts, Some(in_indices));
    // We are adding indices to out utxos, since they will be used as an input utxos in next transaction
    let out_utxos =
        crate::test_util::setup_utxos_2_2_2(out_chain_ids, out_amounts, Some(in_indices));

    let output1 = out_utxos[0].commitment.into_repr().to_bytes_le();
    let output2 = out_utxos[1].commitment.into_repr().to_bytes_le();

    let ext_data = ExtData {
        recipient: RECIPIENT.to_string(),
        relayer: RELAYER.to_string(),
        ext_amount: ext_amount.to_string(),
        fee: Uint128::from(fee),
        encrypted_output1: element_encoder(&output1),
        encrypted_output2: element_encoder(&output2),
    };

    let ext_data_hash = hash_ext_data(ext_data.clone(), ext_amount, fee);

    let custom_roots = Some([zeroes(LEVELS), zeroes(LEVELS)].map(|x| x.to_vec()));
    let (proof, public_inputs) = crate::test_util::setup_zk_circuit_2_2_2(
        public_amount,
        chain_id,
        ext_data_hash.to_vec(),
        in_utxos,
        out_utxos.clone(),
        custom_roots,
        pk_bytes,
    );

    // Deconstructing public inputs
    let (_chain_id, public_amount, root_set, nullifiers, commitments, ext_data_hash) =
        crate::test_util::deconstruct_public_inputs_el_2_2_2(&public_inputs);

    // Constructing proof data
    let root_set = root_set.into_iter().map(|v| v.0).collect();
    let nullifiers = nullifiers.into_iter().map(|v| v.0).collect();
    let commitments = commitments.into_iter().map(|v| v.0).collect();
    let proof_data = ProofData::new(
        proof,
        public_amount.0,
        root_set,
        nullifiers,
        commitments,
        ext_data_hash.0,
    );

    // Should "transact" with success.
    let info = mock_info(CW20_ADDRESS, &[]);
    let deposit_cw20_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: TRANSACTOR.to_string(),
        amount: Uint128::from(10_u128),
        msg: to_binary(&Cw20HookMsg::TransactDeposit {
            proof_data: proof_data,
            ext_data: ext_data,
        })
        .unwrap(),
    });

    // Deposit "10" cw20 tokens.
    let _ = execute(deps.as_mut(), mock_env(), info, deposit_cw20_msg).unwrap();

    // Prepare the "withdraw" data.
    let (pk_bytes, _) = crate::test_util::setup_environment_2_2_2(Curve::Bn254);

    let ext_amount = -5_i128;
    let fee = 2_u128;

    let public_amount = -7_i128;

    let chain_id = compute_chain_id_type(TEST_CHAIN_ID, &CHAIN_TYPE);
    let out_chain_ids = [TEST_CHAIN_ID; 2];
    // After withdrawing -7
    let out_amounts = [100, 200];

    // "in_utxos" become the "out_utxos" of last transact.
    let in_utxos = out_utxos;
    let out_utxos = crate::test_util::setup_utxos_2_2_2(out_chain_ids, out_amounts, None);

    let output1 = out_utxos[0].commitment.into_repr().to_bytes_le();
    let output2 = out_utxos[1].commitment.into_repr().to_bytes_le();
    let ext_data = ExtData {
        recipient: RECIPIENT.to_string(),
        relayer: RELAYER.to_string(),
        ext_amount: ext_amount.to_string(),
        fee: Uint128::from(fee),
        encrypted_output1: element_encoder(&output1),
        encrypted_output2: element_encoder(&output2),
    };

    let ext_data_hash = hash_ext_data(ext_data.clone(), ext_amount, fee);

    let (proof, public_inputs) = crate::test_util::setup_zk_circuit_2_2_2(
        public_amount,
        chain_id,
        ext_data_hash.to_vec(),
        in_utxos,
        out_utxos,
        None,
        pk_bytes,
    );

    // Deconstructing public inputs
    let (_chain_id, public_amount, root_set, nullifiers, commitments, ext_data_hash) =
        crate::test_util::deconstruct_public_inputs_el_2_2_2(&public_inputs);

    // Constructing proof data
    let root_set = root_set.into_iter().map(|v| v.0).collect();
    let nullifiers = nullifiers.into_iter().map(|v| v.0).collect();
    let commitments = commitments.into_iter().map(|v| v.0).collect();

    let proof_data = ProofData::new(
        proof,
        public_amount.0,
        root_set,
        nullifiers,
        commitments,
        ext_data_hash.0,
    );

    // Should fail with "Invalid ext amount".
    let info = mock_info(CW20_ADDRESS, &[]);
    let withdraw_cw20_msg = ExecuteMsg::TransactWithdraw {
        proof_data: proof_data,
        ext_data: ext_data,
    };

    let err = execute(deps.as_mut(), mock_env(), info, withdraw_cw20_msg).unwrap_err();
    assert_eq!(err.to_string(), "Invalid transaction proof".to_string());
}

#[test]
fn test_vanchor_should_not_complete_withdraw_if_out_amount_sum_is_too_small() {
    // Instantiate the "vanchor" contract.
    let mut deps = create_vanchor();

    // Initialize the vanchor
    let (pk_bytes, _) = crate::test_util::setup_environment_2_2_2(Curve::Bn254);
    let ext_amount = 10_i128;
    let fee = 0_u128;

    let public_amount = 10_i128;

    let chain_type = [4, 0];
    let chain_id = compute_chain_id_type(1, &chain_type);
    let in_chain_ids = [chain_id; 2];
    let in_amounts = [0, 0];
    let in_indices = [0, 1];
    let out_chain_ids = [chain_id; 2];
    let out_amounts = [10, 0];

    let in_utxos = crate::test_util::setup_utxos_2_2_2(in_chain_ids, in_amounts, Some(in_indices));
    // We are adding indices to out utxos, since they will be used as an input utxos in next transaction
    let out_utxos =
        crate::test_util::setup_utxos_2_2_2(out_chain_ids, out_amounts, Some(in_indices));

    let output1 = out_utxos[0].commitment.into_repr().to_bytes_le();
    let output2 = out_utxos[1].commitment.into_repr().to_bytes_le();

    let ext_data = ExtData {
        recipient: RECIPIENT.to_string(),
        relayer: RELAYER.to_string(),
        ext_amount: ext_amount.to_string(),
        fee: Uint128::from(fee),
        encrypted_output1: element_encoder(&output1),
        encrypted_output2: element_encoder(&output2),
    };

    let ext_data_hash = hash_ext_data(ext_data.clone(), ext_amount, fee);

    let custom_roots = Some([zeroes(LEVELS), zeroes(LEVELS)].map(|x| x.to_vec()));
    let (proof, public_inputs) = crate::test_util::setup_zk_circuit_2_2_2(
        public_amount,
        chain_id,
        ext_data_hash.to_vec(),
        in_utxos,
        out_utxos.clone(),
        custom_roots,
        pk_bytes,
    );

    // Deconstructing public inputs
    let (_chain_id, public_amount, root_set, nullifiers, commitments, ext_data_hash) =
        crate::test_util::deconstruct_public_inputs_el_2_2_2(&public_inputs);

    // Constructing proof data
    let root_set = root_set.into_iter().map(|v| v.0).collect();
    let nullifiers = nullifiers.into_iter().map(|v| v.0).collect();
    let commitments = commitments.into_iter().map(|v| v.0).collect();
    let proof_data = ProofData::new(
        proof,
        public_amount.0,
        root_set,
        nullifiers,
        commitments,
        ext_data_hash.0,
    );

    // Should "transact" with success.
    let info = mock_info(CW20_ADDRESS, &[]);
    let deposit_cw20_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: TRANSACTOR.to_string(),
        amount: Uint128::from(10_u128),
        msg: to_binary(&Cw20HookMsg::TransactDeposit {
            proof_data: proof_data,
            ext_data: ext_data,
        })
        .unwrap(),
    });

    // Deposit "10" cw20 tokens.
    let _ = execute(deps.as_mut(), mock_env(), info, deposit_cw20_msg).unwrap();

    // Prepare the "withdraw" data.
    let (pk_bytes, _) = crate::test_util::setup_environment_2_2_2(Curve::Bn254);

    let ext_amount = -5_i128;
    let fee = 2_u128;

    let public_amount = -7_i128;

    let chain_id = compute_chain_id_type(TEST_CHAIN_ID, &CHAIN_TYPE);
    let out_chain_ids = [TEST_CHAIN_ID; 2];
    // Too small out amounts
    let out_amounts = [1, 0];

    // "in_utxos" become the "out_utxos" of last transact.
    let in_utxos = out_utxos;
    let out_utxos = crate::test_util::setup_utxos_2_2_2(out_chain_ids, out_amounts, None);

    let output1 = out_utxos[0].commitment.into_repr().to_bytes_le();
    let output2 = out_utxos[1].commitment.into_repr().to_bytes_le();
    let ext_data = ExtData {
        recipient: RECIPIENT.to_string(),
        relayer: RELAYER.to_string(),
        ext_amount: ext_amount.to_string(),
        fee: Uint128::from(fee),
        encrypted_output1: element_encoder(&output1),
        encrypted_output2: element_encoder(&output2),
    };

    let ext_data_hash = hash_ext_data(ext_data.clone(), ext_amount, fee);

    let (proof, public_inputs) = crate::test_util::setup_zk_circuit_2_2_2(
        public_amount,
        chain_id,
        ext_data_hash.to_vec(),
        in_utxos,
        out_utxos,
        None,
        pk_bytes,
    );

    // Deconstructing public inputs
    let (_chain_id, public_amount, root_set, nullifiers, commitments, ext_data_hash) =
        crate::test_util::deconstruct_public_inputs_el_2_2_2(&public_inputs);

    // Constructing proof data
    let root_set = root_set.into_iter().map(|v| v.0).collect();
    let nullifiers = nullifiers.into_iter().map(|v| v.0).collect();
    let commitments = commitments.into_iter().map(|v| v.0).collect();

    let proof_data = ProofData::new(
        proof,
        public_amount.0,
        root_set,
        nullifiers,
        commitments,
        ext_data_hash.0,
    );

    // Should fail with "Invalid ext amount".
    let info = mock_info(CW20_ADDRESS, &[]);
    let withdraw_cw20_msg = ExecuteMsg::TransactWithdraw {
        proof_data: proof_data,
        ext_data: ext_data,
    };

    let err = execute(deps.as_mut(), mock_env(), info, withdraw_cw20_msg).unwrap_err();
    assert_eq!(err.to_string(), "Invalid transaction proof".to_string());
}

#[test]
fn test_vanchor_should_not_be_able_to_double_spend() {
    // Instantiate the "vanchor" contract.
    let mut deps = create_vanchor();

    // Initialize the vanchor
    let (pk_bytes, _) = crate::test_util::setup_environment_2_2_2(Curve::Bn254);
    let ext_amount = 10_i128;
    let fee = 0_u128;

    let public_amount = 10_i128;

    let chain_type = [4, 0];
    let chain_id = compute_chain_id_type(1, &chain_type);
    let in_chain_ids = [chain_id; 2];
    let in_amounts = [0, 0];
    let in_indices = [0, 1];
    let out_chain_ids = [chain_id; 2];
    let out_amounts = [10, 0];

    let in_utxos = crate::test_util::setup_utxos_2_2_2(in_chain_ids, in_amounts, Some(in_indices));
    // We are adding indices to out utxos, since they will be used as an input utxos in next transaction
    let out_utxos =
        crate::test_util::setup_utxos_2_2_2(out_chain_ids, out_amounts, Some(in_indices));

    let output1 = out_utxos[0].commitment.into_repr().to_bytes_le();
    let output2 = out_utxos[1].commitment.into_repr().to_bytes_le();

    let ext_data = ExtData {
        recipient: RECIPIENT.to_string(),
        relayer: RELAYER.to_string(),
        ext_amount: ext_amount.to_string(),
        fee: Uint128::from(fee),
        encrypted_output1: element_encoder(&output1),
        encrypted_output2: element_encoder(&output2),
    };

    let ext_data_hash = hash_ext_data(ext_data.clone(), ext_amount, fee);

    let custom_roots = Some([zeroes(LEVELS), zeroes(LEVELS)].map(|x| x.to_vec()));
    let (proof, public_inputs) = crate::test_util::setup_zk_circuit_2_2_2(
        public_amount,
        chain_id,
        ext_data_hash.to_vec(),
        in_utxos,
        out_utxos.clone(),
        custom_roots,
        pk_bytes,
    );

    // Deconstructing public inputs
    let (_chain_id, public_amount, root_set, nullifiers, commitments, ext_data_hash) =
        crate::test_util::deconstruct_public_inputs_el_2_2_2(&public_inputs);

    // Constructing proof data
    let root_set = root_set.into_iter().map(|v| v.0).collect();
    let nullifiers = nullifiers.into_iter().map(|v| v.0).collect();
    let commitments = commitments.into_iter().map(|v| v.0).collect();
    let proof_data = ProofData::new(
        proof,
        public_amount.0,
        root_set,
        nullifiers,
        commitments,
        ext_data_hash.0,
    );

    // Should "transact" with success.
    let info = mock_info(CW20_ADDRESS, &[]);
    let deposit_cw20_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: TRANSACTOR.to_string(),
        amount: Uint128::from(10_u128),
        msg: to_binary(&Cw20HookMsg::TransactDeposit {
            proof_data: proof_data,
            ext_data: ext_data,
        })
        .unwrap(),
    });

    // Deposit "10" cw20 tokens.
    let _ = execute(deps.as_mut(), mock_env(), info, deposit_cw20_msg).unwrap();

    // Prepare the "withdraw" data.
    let (pk_bytes, _) = crate::test_util::setup_environment_2_2_2(Curve::Bn254);

    let ext_amount = -5_i128;
    let fee = 2_u128;

    let public_amount = -7_i128;

    let chain_id = compute_chain_id_type(TEST_CHAIN_ID, &CHAIN_TYPE);
    let out_chain_ids = [TEST_CHAIN_ID; 2];
    // After withdraw -7
    let out_amounts = [1, 2];

    // "in_utxos" become the "out_utxos" of last transact.
    let in_utxos = out_utxos;
    let out_utxos = crate::test_util::setup_utxos_2_2_2(out_chain_ids, out_amounts, None);

    let output1 = out_utxos[0].commitment.into_repr().to_bytes_le();
    let output2 = out_utxos[1].commitment.into_repr().to_bytes_le();
    let ext_data = ExtData {
        recipient: RECIPIENT.to_string(),
        relayer: RELAYER.to_string(),
        ext_amount: ext_amount.to_string(),
        fee: Uint128::from(fee),
        encrypted_output1: element_encoder(&output1),
        encrypted_output2: element_encoder(&output2),
    };

    let ext_data_hash = hash_ext_data(ext_data.clone(), ext_amount, fee);

    let (proof, public_inputs) = crate::test_util::setup_zk_circuit_2_2_2(
        public_amount,
        chain_id,
        ext_data_hash.to_vec(),
        in_utxos,
        out_utxos,
        None,
        pk_bytes,
    );

    // Deconstructing public inputs
    let (_chain_id, public_amount, root_set, nullifiers, commitments, ext_data_hash) =
        crate::test_util::deconstruct_public_inputs_el_2_2_2(&public_inputs);

    // Constructing proof data
    let root_set = root_set.into_iter().map(|v| v.0).collect();
    let nullifiers = nullifiers.into_iter().map(|v| v.0).collect();
    let commitments = commitments.into_iter().map(|v| v.0).collect();

    let proof_data = ProofData::new(
        proof,
        public_amount.0,
        root_set,
        nullifiers,
        commitments,
        ext_data_hash.0,
    );

    // Should succeed first, & fail second attempt.
    let info = mock_info(CW20_ADDRESS, &[]);
    let withdraw_cw20_msg = ExecuteMsg::TransactWithdraw {
        proof_data: proof_data,
        ext_data: ext_data,
    };

    // Should success since first attempt.
    let _ = execute(
        deps.as_mut(),
        mock_env(),
        info.clone(),
        withdraw_cw20_msg.clone(),
    )
    .unwrap();

    // Should fail since second attemp.
    let err = execute(deps.as_mut(), mock_env(), info, withdraw_cw20_msg).unwrap_err();
    assert_eq!(
        err.to_string(),
        "Invalid nullifier that is already used".to_string()
    );
}

#[test]
fn test_vanchor_should_complete_16x2_transaction_with_deposit_cw20() {
    // Instantiate the "vanchor" contract.
    let mut deps = create_vanchor();

    // Initialize the vanchor
    let (pk_bytes, _) = crate::test_util::setup_environment_2_16_2(Curve::Bn254);
    let ext_amount = 160_i128;
    let fee = 0_u128;

    let public_amount = 160_i128;

    let chain_id = compute_chain_id_type(1, &CHAIN_TYPE);
    let in_chain_ids = [chain_id; 16];
    let in_amounts = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    let in_indices = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
    let out_chain_ids = [chain_id; 2];
    let out_amounts = [160, 0];
    let out_indices = [0, 1];

    let in_utxos = crate::test_util::setup_utxos_2_16_2(in_chain_ids, in_amounts, Some(in_indices));
    // We are adding indices to out utxos, since they will be used as an input utxos in next transaction
    let out_utxos =
        crate::test_util::setup_utxos_2_2_2(out_chain_ids, out_amounts, Some(out_indices));

    let output1 = out_utxos[0].commitment.into_repr().to_bytes_le();
    let output2 = out_utxos[1].commitment.into_repr().to_bytes_le();

    let ext_data = ExtData {
        recipient: RECIPIENT.to_string(),
        relayer: RELAYER.to_string(),
        ext_amount: ext_amount.to_string(),
        fee: Uint128::from(fee),
        encrypted_output1: element_encoder(&output1),
        encrypted_output2: element_encoder(&output2),
    };

    let ext_data_hash = hash_ext_data(ext_data.clone(), ext_amount, fee);

    let custom_roots = Some([zeroes(LEVELS), zeroes(LEVELS)].map(|x| x.to_vec()));
    let (proof, public_inputs) = crate::test_util::setup_zk_circuit_2_16_2(
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
        crate::test_util::deconstruct_public_inputs_el_2_16_2(&public_inputs);

    // Constructing proof data
    let root_set = root_set.into_iter().map(|v| v.0).collect();
    let nullifiers = nullifiers.into_iter().map(|v| v.0).collect();
    let commitments = commitments.into_iter().map(|v| v.0).collect();
    let proof_data = ProofData::new(
        proof,
        public_amount.0,
        root_set,
        nullifiers,
        commitments,
        ext_data_hash.0,
    );

    // Should "transact" with success.
    let info = mock_info(CW20_ADDRESS, &[]);
    let deposit_cw20_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: TRANSACTOR.to_string(),
        amount: Uint128::from(160_u128),
        msg: to_binary(&Cw20HookMsg::TransactDeposit {
            proof_data: proof_data,
            ext_data: ext_data,
        })
        .unwrap(),
    });

    let response = execute(deps.as_mut(), mock_env(), info, deposit_cw20_msg).unwrap();
    assert_eq!(
        response.attributes,
        vec![attr("method", "transact_deposit"), attr("ext_amt", "160"),]
    );
}

#[test]
fn test_vanchor_should_complete_16x2_transaction_with_withdraw_cw20() {
    // Instantiate the "vanchor" contract.
    let mut deps = create_vanchor();

    // Initialize the vanchor
    let (pk_bytes, _) = crate::test_util::setup_environment_2_16_2(Curve::Bn254);
    let ext_amount = 160_i128;
    let fee = 0_u128;

    let public_amount = 160_i128;

    let chain_id = compute_chain_id_type(1, &CHAIN_TYPE);
    let in_chain_ids = [chain_id; 16];
    let in_amounts = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    let in_indices = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
    let out_chain_ids = [chain_id; 2];
    let out_amounts = [160, 0];
    let out_indices = [0, 1];

    let in_utxos = crate::test_util::setup_utxos_2_16_2(in_chain_ids, in_amounts, Some(in_indices));
    // We are adding indices to out utxos, since they will be used as an input utxos in next transaction
    let out_utxos =
        crate::test_util::setup_utxos_2_2_2(out_chain_ids, out_amounts, Some(out_indices));

    let output1 = out_utxos[0].commitment.into_repr().to_bytes_le();
    let output2 = out_utxos[1].commitment.into_repr().to_bytes_le();

    let ext_data = ExtData {
        recipient: RECIPIENT.to_string(),
        relayer: RELAYER.to_string(),
        ext_amount: ext_amount.to_string(),
        fee: Uint128::from(fee),
        encrypted_output1: element_encoder(&output1),
        encrypted_output2: element_encoder(&output2),
    };

    let ext_data_hash = hash_ext_data(ext_data.clone(), ext_amount, fee);

    let custom_roots = Some([zeroes(LEVELS), zeroes(LEVELS)].map(|x| x.to_vec()));
    let (proof, public_inputs) = crate::test_util::setup_zk_circuit_2_16_2(
        public_amount,
        chain_id,
        ext_data_hash.to_vec(),
        in_utxos,
        out_utxos.clone(),
        custom_roots,
        pk_bytes,
    );

    // Deconstructing public inputs
    let (_chain_id, public_amount, root_set, nullifiers, commitments, ext_data_hash) =
        crate::test_util::deconstruct_public_inputs_el_2_16_2(&public_inputs);

    // Constructing proof data
    let root_set = root_set.into_iter().map(|v| v.0).collect();
    let nullifiers = nullifiers.into_iter().map(|v| v.0).collect();
    let commitments = commitments.into_iter().map(|v| v.0).collect();
    let proof_data = ProofData::new(
        proof,
        public_amount.0,
        root_set,
        nullifiers,
        commitments,
        ext_data_hash.0,
    );

    // Should "transact" with success.
    let info = mock_info(CW20_ADDRESS, &[]);
    let deposit_cw20_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: TRANSACTOR.to_string(),
        amount: Uint128::from(160_u128),
        msg: to_binary(&Cw20HookMsg::TransactDeposit {
            proof_data: proof_data,
            ext_data: ext_data,
        })
        .unwrap(),
    });

    // Deposit "10" cw20 tokens.
    let _ = execute(deps.as_mut(), mock_env(), info, deposit_cw20_msg).unwrap();

    // Prepare the "withdraw" data.
    let (pk_bytes, _) = crate::test_util::setup_environment_2_2_2(Curve::Bn254);

    let ext_amount = -50_i128;
    let fee = 30_u128;

    let public_amount = -80_i128;

    let chain_id = compute_chain_id_type(TEST_CHAIN_ID, &CHAIN_TYPE);
    let out_chain_ids = [TEST_CHAIN_ID; 2];
    // After withdrawing -7
    let out_amounts = [40, 40];

    // "in_utxos" become the "out_utxos" of last transact.
    let in_utxos = out_utxos;
    let out_utxos = crate::test_util::setup_utxos_2_2_2(out_chain_ids, out_amounts, None);

    let output1 = out_utxos[0].commitment.into_repr().to_bytes_le();
    let output2 = out_utxos[1].commitment.into_repr().to_bytes_le();
    let ext_data = ExtData {
        recipient: RECIPIENT.to_string(),
        relayer: RELAYER.to_string(),
        ext_amount: ext_amount.to_string(),
        fee: Uint128::from(fee),
        encrypted_output1: element_encoder(&output1),
        encrypted_output2: element_encoder(&output2),
    };

    let ext_data_hash = hash_ext_data(ext_data.clone(), ext_amount, fee);

    let (proof, public_inputs) = crate::test_util::setup_zk_circuit_2_2_2(
        public_amount,
        chain_id,
        ext_data_hash.to_vec(),
        in_utxos,
        out_utxos,
        None,
        pk_bytes,
    );

    // Deconstructing public inputs
    let (_chain_id, public_amount, root_set, nullifiers, commitments, ext_data_hash) =
        crate::test_util::deconstruct_public_inputs_el_2_2_2(&public_inputs);

    // Constructing proof data
    let root_set = root_set.into_iter().map(|v| v.0).collect();
    let nullifiers = nullifiers.into_iter().map(|v| v.0).collect();
    let commitments = commitments.into_iter().map(|v| v.0).collect();

    let proof_data = ProofData::new(
        proof,
        public_amount.0,
        root_set,
        nullifiers,
        commitments,
        ext_data_hash.0,
    );

    // Should "transact" with success.
    let info = mock_info(CW20_ADDRESS, &[]);
    let withdraw_cw20_msg = ExecuteMsg::TransactWithdraw {
        proof_data: proof_data,
        ext_data: ext_data,
    };

    // Withdraw "7" cw20 tokens.
    let response = execute(deps.as_mut(), mock_env(), info, withdraw_cw20_msg).unwrap();
    assert_eq!(
        response.attributes,
        vec![attr("method", "transact_withdraw"), attr("ext_amt", "-50"),]
    );
}

#[test]
fn test_vanchor_wrap_native() {
    let mut deps = create_vanchor();

    let wrap_amt = Uint128::from(100_u128);

    let info = mock_info("anyone", &coins(wrap_amt.u128(), "uusd"));
    let wrap_native_msg = ExecuteMsg::WrapNative {
        amount: Uint128::from(wrap_amt),
        is_deposit: false,
    };
    let response = execute(deps.as_mut(), mock_env(), info, wrap_native_msg).unwrap();

    assert_eq!(response.messages.len(), 1);
    assert_eq!(
        response.attributes,
        vec![
            attr("method", "wrap_native"),
            attr("denom", "uusd"),
            attr("amount", wrap_amt.to_string()),
        ]
    );
}

#[test]
fn test_vanchor_unwrap_native() {
    let mut deps = create_vanchor();

    let unwrap_amt = Uint128::from(100_u128);

    let info = mock_info("anyone", &[]);
    let unwrap_native_msg = ExecuteMsg::UnwrapNative {
        amount: unwrap_amt,
        recipient: None,
    };
    let response = execute(deps.as_mut(), mock_env(), info, unwrap_native_msg).unwrap();

    assert_eq!(response.messages.len(), 1);
    assert_eq!(
        response.attributes,
        vec![
            attr("method", "unwrap_native"),
            attr("amount", unwrap_amt.to_string()),
        ]
    );
}

#[test]
fn test_vanchor_wrap_token() {
    let mut deps = create_vanchor();

    let wrap_amt = Uint128::from(100_u128);
    let wrap_token = "recv_token";

    let info = mock_info(wrap_token, &[]);
    let wrap_token_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "anyone".to_string(),
        amount: wrap_amt,
        msg: to_binary(&Cw20HookMsg::WrapToken { is_deposit: false }).unwrap(),
    });
    let response = execute(deps.as_mut(), mock_env(), info, wrap_token_msg).unwrap();

    assert_eq!(response.messages.len(), 1);
    assert_eq!(
        response.attributes,
        vec![
            attr("method", "wrap_token"),
            attr("token", wrap_token.to_string()),
            attr("amount", wrap_amt.to_string()),
        ]
    );
}

#[test]
fn test_vanchor_unwrap_into_token() {
    let mut deps = create_vanchor();

    let unwrap_amt = 100_u128;
    let recv_token = "recv_token";

    let info = mock_info("anyone", &[]);
    let unwrap_into_token_msg = ExecuteMsg::UnwrapIntoToken {
        token_addr: recv_token.to_string(),
        amount: Uint128::from(unwrap_amt),
        recipient: None,
    };
    let response = execute(deps.as_mut(), mock_env(), info, unwrap_into_token_msg).unwrap();

    assert_eq!(response.messages.len(), 1);
    assert_eq!(
        response.attributes,
        vec![
            attr("method", "unwrap_into_token"),
            attr("token", recv_token),
            attr("amount", unwrap_amt.to_string()),
        ]
    );
}

#[test]
fn test_vanchor_wrap_and_deposit_cw20() {
    // Instantiate the "vanchor" contract.
    let mut deps = create_vanchor();

    // Initialize the vanchor
    let (pk_bytes, _) = crate::test_util::setup_environment_2_2_2(Curve::Bn254);
    let ext_amount = 10_i128;
    let fee = 0_u128;

    let public_amount = 10_i128;

    let chain_type = [4, 0];
    let chain_id = compute_chain_id_type(1, &chain_type);
    let in_chain_ids = [chain_id; 2];
    let in_amounts = [0, 0];
    let in_indices = [0, 1];
    let out_chain_ids = [chain_id; 2];
    let out_amounts = [10, 0];

    let in_utxos = crate::test_util::setup_utxos_2_2_2(in_chain_ids, in_amounts, Some(in_indices));
    // We are adding indices to out utxos, since they will be used as an input utxos in next transaction
    let out_utxos =
        crate::test_util::setup_utxos_2_2_2(out_chain_ids, out_amounts, Some(in_indices));

    let output1 = out_utxos[0].commitment.into_repr().to_bytes_le();
    let output2 = out_utxos[1].commitment.into_repr().to_bytes_le();

    let ext_data = ExtData {
        recipient: RECIPIENT.to_string(),
        relayer: RELAYER.to_string(),
        ext_amount: ext_amount.to_string(),
        fee: Uint128::from(fee),
        encrypted_output1: element_encoder(&output1),
        encrypted_output2: element_encoder(&output2),
    };

    let ext_data_hash = hash_ext_data(ext_data.clone(), ext_amount, fee);

    let custom_roots = Some([zeroes(LEVELS), zeroes(LEVELS)].map(|x| x.to_vec()));
    let (proof, public_inputs) = crate::test_util::setup_zk_circuit_2_2_2(
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
        crate::test_util::deconstruct_public_inputs_el_2_2_2(&public_inputs);

    // Constructing proof data
    let root_set = root_set.into_iter().map(|v| v.0).collect();
    let nullifiers = nullifiers.into_iter().map(|v| v.0).collect();
    let commitments = commitments.into_iter().map(|v| v.0).collect();
    let proof_data = ProofData::new(
        proof,
        public_amount.0,
        root_set,
        nullifiers,
        commitments,
        ext_data_hash.0,
    );

    // Should "transact" with success.
    let info = mock_info("any-cw20", &[]);
    let deposit_cw20_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: TRANSACTOR.to_string(),
        amount: Uint128::from(10_u128),
        msg: to_binary(&Cw20HookMsg::TransactDepositWrap {
            proof_data: proof_data,
            ext_data: ext_data,
        })
        .unwrap(),
    });

    let response = execute(deps.as_mut(), mock_env(), info, deposit_cw20_msg).unwrap();
    assert_eq!(
        response.attributes,
        vec![
            attr("method", "transact_deposit_wrap_cw20"),
            attr("ext_amt", "10"),
        ]
    );
    assert_eq!(response.messages.len(), 1);
}

#[test]
fn test_vanchor_withdraw_and_unwrap_native() {
    // Instantiate the "vanchor" contract.
    let mut deps = create_vanchor();

    // Initialize the vanchor
    let (pk_bytes, _) = crate::test_util::setup_environment_2_2_2(Curve::Bn254);
    let ext_amount = 10_i128;
    let fee = 0_u128;

    let public_amount = 10_i128;

    let chain_type = [4, 0];
    let chain_id = compute_chain_id_type(1, &chain_type);
    let in_chain_ids = [chain_id; 2];
    let in_amounts = [0, 0];
    let in_indices = [0, 1];
    let out_chain_ids = [chain_id; 2];
    let out_amounts = [10, 0];

    let in_utxos = crate::test_util::setup_utxos_2_2_2(in_chain_ids, in_amounts, Some(in_indices));
    // We are adding indices to out utxos, since they will be used as an input utxos in next transaction
    let out_utxos =
        crate::test_util::setup_utxos_2_2_2(out_chain_ids, out_amounts, Some(in_indices));

    let output1 = out_utxos[0].commitment.into_repr().to_bytes_le();
    let output2 = out_utxos[1].commitment.into_repr().to_bytes_le();

    let ext_data = ExtData {
        recipient: RECIPIENT.to_string(),
        relayer: RELAYER.to_string(),
        ext_amount: ext_amount.to_string(),
        fee: Uint128::from(fee),
        encrypted_output1: element_encoder(&output1),
        encrypted_output2: element_encoder(&output2),
    };

    let ext_data_hash = hash_ext_data(ext_data.clone(), ext_amount, fee);

    let custom_roots = Some([zeroes(LEVELS), zeroes(LEVELS)].map(|x| x.to_vec()));
    let (proof, public_inputs) = crate::test_util::setup_zk_circuit_2_2_2(
        public_amount,
        chain_id,
        ext_data_hash.to_vec(),
        in_utxos,
        out_utxos.clone(),
        custom_roots,
        pk_bytes,
    );

    // Deconstructing public inputs
    let (_chain_id, public_amount, root_set, nullifiers, commitments, ext_data_hash) =
        crate::test_util::deconstruct_public_inputs_el_2_2_2(&public_inputs);

    // Constructing proof data
    let root_set = root_set.into_iter().map(|v| v.0).collect();
    let nullifiers = nullifiers.into_iter().map(|v| v.0).collect();
    let commitments = commitments.into_iter().map(|v| v.0).collect();
    let proof_data = ProofData::new(
        proof,
        public_amount.0,
        root_set,
        nullifiers,
        commitments,
        ext_data_hash.0,
    );

    println!("proof_data:{:?}\n", proof_data);
    println!("ext_data:{:?}", ext_data);

    // Should "transact" with success.
    let info = mock_info(CW20_ADDRESS, &[]);
    let deposit_cw20_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: TRANSACTOR.to_string(),
        amount: Uint128::from(10_u128),
        msg: to_binary(&Cw20HookMsg::TransactDeposit {
            proof_data: proof_data,
            ext_data: ext_data,
        })
        .unwrap(),
    });

    // Deposit "10" cw20 tokens.
    let _ = execute(deps.as_mut(), mock_env(), info, deposit_cw20_msg).unwrap();

    // Prepare the "withdraw" data.
    let (pk_bytes, _) = crate::test_util::setup_environment_2_2_2(Curve::Bn254);

    let ext_amount = -5_i128;
    let fee = 2_u128;

    let public_amount = -7_i128;

    let chain_id = compute_chain_id_type(TEST_CHAIN_ID, &CHAIN_TYPE);
    let out_chain_ids = [TEST_CHAIN_ID; 2];
    // After withdrawing -7
    let out_amounts = [1, 2];

    // "in_utxos" become the "out_utxos" of last transact.
    let in_utxos = out_utxos;
    let out_utxos = crate::test_util::setup_utxos_2_2_2(out_chain_ids, out_amounts, None);

    let output1 = out_utxos[0].commitment.into_repr().to_bytes_le();
    let output2 = out_utxos[1].commitment.into_repr().to_bytes_le();
    let ext_data = ExtData {
        recipient: RECIPIENT.to_string(),
        relayer: RELAYER.to_string(),
        ext_amount: ext_amount.to_string(),
        fee: Uint128::from(fee),
        encrypted_output1: element_encoder(&output1),
        encrypted_output2: element_encoder(&output2),
    };

    let ext_data_hash = hash_ext_data(ext_data.clone(), ext_amount, fee);

    let (proof, public_inputs) = crate::test_util::setup_zk_circuit_2_2_2(
        public_amount,
        chain_id,
        ext_data_hash.to_vec(),
        in_utxos,
        out_utxos,
        None,
        pk_bytes,
    );

    // Deconstructing public inputs
    let (_chain_id, public_amount, root_set, nullifiers, commitments, ext_data_hash) =
        crate::test_util::deconstruct_public_inputs_el_2_2_2(&public_inputs);

    // Constructing proof data
    let root_set = root_set.into_iter().map(|v| v.0).collect();
    let nullifiers = nullifiers.into_iter().map(|v| v.0).collect();
    let commitments = commitments.into_iter().map(|v| v.0).collect();

    let proof_data = ProofData::new(
        proof,
        public_amount.0,
        root_set,
        nullifiers,
        commitments,
        ext_data_hash.0,
    );

    println!("proof_data:{:?}\n", proof_data);
    println!("ext_data:{:?}", ext_data);

    // Should "transact" with success.
    let info = mock_info(CW20_ADDRESS, &[]);
    let withdraw_cw20_msg = ExecuteMsg::TransactWithdrawUnwrap {
        proof_data: proof_data,
        ext_data: ext_data,
        token_addr: None,
    };

    // Withdraw "7" cw20 tokens.
    let response = execute(deps.as_mut(), mock_env(), info, withdraw_cw20_msg).unwrap();
    assert_eq!(
        response.attributes,
        vec![
            attr("method", "transact_withdraw_unwrap"),
            attr("ext_amt", "-5"),
        ]
    );
}

#[test]
fn test_vanchor_set_handler() {
    let new_handler: &str = "new-handler-address";
    let nonce: u32 = 2u32;

    let mut deps = create_vanchor();

    // Fails to "set handler" if tx sender is not current handler addr
    let info = mock_info("anyone", &[]);
    let set_handler_msg = ExecuteMsg::SetHandler {
        handler: new_handler.to_string(),
        nonce,
    };
    let err = execute(deps.as_mut(), mock_env(), info, set_handler_msg).unwrap_err();
    assert_eq!(err, ContractError::Unauthorized {});

    // Fails to "set handler" if "nonce" is too big or small
    let info = mock_info(HANDLER, &[]);
    let set_handler_msg = ExecuteMsg::SetHandler {
        handler: new_handler.to_string(),
        nonce: nonce + 2000,
    };
    let err = execute(deps.as_mut(), mock_env(), info, set_handler_msg).unwrap_err();
    assert_eq!(err, ContractError::InvalidNonce);

    // Succeed to "set handler"
    let info = mock_info(HANDLER, &[]);
    let set_handler_msg = ExecuteMsg::SetHandler {
        handler: new_handler.to_string(),
        nonce,
    };
    let res = execute(deps.as_mut(), mock_env(), info, set_handler_msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("method", "set_handler"),
            attr("handler", new_handler),
            attr("nonce", nonce.to_string()),
        ]
    );
}
