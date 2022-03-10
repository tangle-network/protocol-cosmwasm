//! This integration test tries to run and call the generated wasm.

use cosmwasm_std::{attr, Coin, Response, Uint128, Uint256};
use cosmwasm_vm::testing::{
    execute, instantiate, mock_env, mock_info, mock_instance_with_gas_limit,
};
use protocol_cosmwasm::anchor::{DepositMsg, ExecuteMsg, InstantiateMsg};

use ark_bn254::Fr;
use ark_crypto_primitives::CRH as CRHTrait;
use ark_ff::PrimeField;
use ark_ff::{BigInteger, Field};
use ark_std::One;
use arkworks_gadgets::poseidon::CRH;
use arkworks_utils::utils::bn254_x5_5::get_poseidon_bn254_x5_5;
type PoseidonCRH5 = CRH<Fr>;

// This line will test the output of cargo wasm
// static WASM: &[u8] = include_bytes!("../../../artifacts/cosmwasm_anchor.wasm");

// You can uncomment this line instead to test productionified build from rust-optimizer
// static WASM: &[u8] =
//     include_bytes!("../../../target/wasm32-unknown-unknown/release/cosmwasm_anchor.wasm");

// For the github CI, we copy the wasm file manually & import here.
static WASM: &[u8] = include_bytes!("./cosmwasm_anchor.wasm");

#[test]
fn integration_test_instantiate_anchor() {
    // "Gas_limit" should be set high manually, since the low value can cause "GasDepletion" error.
    let mut deps = mock_instance_with_gas_limit(WASM, 100_000_000);

    let msg = InstantiateMsg {
        max_edges: 0,
        chain_id: 1,
        levels: 0,
        deposit_size: Uint128::from(1_000_000_u128),
    };

    let info = mock_info("anyone", &[]);
    let response: Response = instantiate(&mut deps, mock_env(), info, msg).unwrap();

    assert_eq!(
        response.attributes,
        vec![attr("method", "instantiate"), attr("owner", "anyone"),]
    );
}

#[test]
fn integration_test_anchor_success_workflow() {
    let mut deps = mock_instance_with_gas_limit(WASM, 200_000_000);

    // Initialize the contract
    let env = mock_env();
    let info = mock_info("anyone", &[]);
    let instantiate_msg = InstantiateMsg {
        max_edges: 2,
        chain_id: 1,
        levels: 30,
        deposit_size: Uint128::from(1_000_000_u128),
    };

    let _res: Response = instantiate(&mut deps, env, info, instantiate_msg).unwrap();

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
    let info = mock_info("depositor", &[Coin::new(1_000_000_u128, "uusd")]);
    let deposit_msg = ExecuteMsg::Deposit(DepositMsg {
        from: None,
        commitment: Some(element),
        value: Uint256::from(0_u128),
    });

    let response: Response = execute(&mut deps, mock_env(), info, deposit_msg).unwrap();
    assert_eq!(
        response.attributes,
        vec![attr("method", "deposit"), attr("result", "0")]
    );

    // Didn't add the "withdraw" test since it needs the input params from "test_util" module.
}
