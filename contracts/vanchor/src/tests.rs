use ark_ff::BigInteger;
use ark_ff::PrimeField;
use arkworks_setups::Curve;
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{attr, to_binary, Uint128, Uint256};
use cw20::Cw20ReceiveMsg;
use sp_core::hashing::keccak_256;

use crate::contract::{compute_chain_id_type, execute, instantiate};
use protocol_cosmwasm::vanchor::{
    Cw20HookMsg, ExecuteMsg, ExtData, InstantiateMsg, ProofData, UpdateConfigMsg,
};

const CHAIN_ID: u64 = 1;
const MAX_EDGES: u32 = 2;
const LEVELS: u32 = 30;
const MAX_DEPOSIT_AMT: u128 = 40;
const MIN_WITHDRAW_AMT: u128 = 0;
const MAX_EXT_AMT: u128 = 20;
const MAX_FEE: u128 = 10;
const CW20_ADDRESS: &str = "terra1fex9f78reuwhfsnc8sun6mz8rl9zwqh03fhwf3";

fn element_encoder(v: &[u8]) -> [u8; 32] {
    let mut output = [0u8; 32];
    output.iter_mut().zip(v).for_each(|(b1, b2)| *b1 = *b2);
    output
}

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {
        chain_id: CHAIN_ID,
        max_edges: MAX_EDGES,
        levels: LEVELS,
        max_deposit_amt: Uint256::from(MAX_DEPOSIT_AMT),
        min_withdraw_amt: Uint256::from(MIN_WITHDRAW_AMT),
        max_ext_amt: Uint256::from(MAX_EXT_AMT),
        max_fee: Uint256::from(MAX_FEE),
        cw20_address: CW20_ADDRESS.to_string(),
    };
    let info = mock_info("creator", &[]);

    // we can just call .unwrap() to assert this was a success
    let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());
}

#[test]
fn test_vanchor_update_config() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {
        chain_id: CHAIN_ID,
        max_edges: MAX_EDGES,
        levels: LEVELS,
        max_deposit_amt: Uint256::from(MAX_DEPOSIT_AMT),
        min_withdraw_amt: Uint256::from(MIN_WITHDRAW_AMT),
        max_ext_amt: Uint256::from(MAX_EXT_AMT),
        max_fee: Uint256::from(MAX_FEE),
        cw20_address: CW20_ADDRESS.to_string(),
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
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {
        chain_id: CHAIN_ID,
        max_edges: MAX_EDGES,
        levels: LEVELS,
        max_deposit_amt: Uint256::from(MAX_DEPOSIT_AMT),
        min_withdraw_amt: Uint256::from(MIN_WITHDRAW_AMT),
        max_ext_amt: Uint256::from(MAX_EXT_AMT),
        max_fee: Uint256::from(MAX_FEE),
        cw20_address: CW20_ADDRESS.to_string(),
    };
    let info = mock_info("creator", &[]);

    // we can just call .unwrap() to assert this was a success
    let _ = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    // Initialize the vanchor
    let (pk_bytes, _) = crate::test_util::setup_environment(Curve::Bn254);
    let transactor_bytes = [1u8; 32];
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
    let mut public_amount_le_bytes: [u8; 16] = [0; 16];
    public_amount_le_bytes.copy_from_slice(&public_amount.0.split_at(16).0);
    let proof_data = ProofData::new(
        proof,
        Uint256::from(u128::from_le_bytes(public_amount_le_bytes)),
        root_set,
        nullifiers,
        commitments,
        ext_data_hash.0,
    );

    // Should "transact" with success.
    let info = mock_info(CW20_ADDRESS, &[]);
    let deposit_cw20_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: hex::encode(transactor_bytes),
        amount: Uint128::from(u128::from_le_bytes(public_amount_le_bytes)),
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
