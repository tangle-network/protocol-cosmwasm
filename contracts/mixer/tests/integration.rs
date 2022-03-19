//! This integration test tries to run and call the generated wasm.

use cosmwasm_std::{attr, from_binary, to_binary, Coin, Response, Uint128, Uint256};
use cosmwasm_vm::testing::{
    execute, instantiate, mock_env, mock_info, mock_instance_with_gas_limit, query,
};
use cw20::Cw20ReceiveMsg;
use protocol_cosmwasm::mixer::{
    Cw20HookMsg, DepositMsg, ExecuteMsg, InfoResponse, InstantiateMsg, QueryMsg,
};

use ark_bn254::Fr;
use ark_crypto_primitives::CRH as CRHTrait;
use ark_ff::PrimeField;
use ark_ff::{BigInteger, Field};
use ark_std::One;
use arkworks_native_gadgets::poseidon::{FieldHasher, Poseidon};
use arkworks_setups::common::setup_params;
use arkworks_setups::Curve;

// This line will test the output of cargo wasm
// static WASM: &[u8] = include_bytes!("../../../artifacts/cosmwasm_mixer.wasm");

// You can uncomment this line instead to test productionified build from rust-optimizer
// static WASM: &[u8] =
//     include_bytes!("../../../target/wasm32-unknown-unknown/release/cosmwasm_mixer.wasm");

// For the github CI, we copy the wasm file manually & import here.
static WASM: &[u8] = include_bytes!("./cosmwasm_mixer.wasm");

#[test]
fn integration_test_instantiate_mixer() {
    // "Gas_limit" should be set high manually, since the low value can cause "GasDepletion" error.
    let mut deps = mock_instance_with_gas_limit(WASM, 100_000_000);

    let cw20_address = "terra1fex9f78reuwhfsnc8sun6mz8rl9zwqh03fhwf3".to_string();

    let msg = InstantiateMsg {
        merkletree_levels: 30,
        deposit_size: Uint128::from(1_000_000_u128),
        cw20_address: Some(cw20_address.clone()),
    };

    let info = mock_info("anyone", &[]);
    let response: Response = instantiate(&mut deps, mock_env(), info, msg).unwrap();

    assert_eq!(
        response.attributes,
        vec![attr("method", "instantiate"), attr("owner", "anyone"),]
    );

    let query = query(&mut deps, mock_env(), QueryMsg::GetCw20Address {}).unwrap();
    let info: InfoResponse = from_binary(&query).unwrap();
    assert_eq!(info.cw20_address, cw20_address);
}

#[test]
fn integration_test_mixer_deposit_native() {
    let mut deps = mock_instance_with_gas_limit(WASM, 200_000_000);

    // Initialize the contract
    let env = mock_env();
    let info = mock_info("anyone", &[]);
    let instantiate_msg = InstantiateMsg {
        merkletree_levels: 30,
        deposit_size: Uint128::from(1_000_000_u128),
        cw20_address: None,
    };

    let _res: Response = instantiate(&mut deps, env, info, instantiate_msg).unwrap();

    // Initialize the mixer
    let params = setup_params(Curve::Bn254, 5, 3);
    let poseidon = Poseidon::new(params);
    let res = poseidon.hash_two(&Fr::one(), &Fr::one()).unwrap();
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
}

#[test]
fn integration_test_mixer_deposit_cw20() {
    let cw20_address = "terra1fex9f78reuwhfsnc8sun6mz8rl9zwqh03fhwf3".to_string();

    let mut deps = mock_instance_with_gas_limit(WASM, 200_000_000);

    // Initialize the contract
    let env = mock_env();
    let info = mock_info("anyone", &[]);
    let instantiate_msg = InstantiateMsg {
        merkletree_levels: 30,
        deposit_size: Uint128::from(1_000_000_u128),
        cw20_address: Some(cw20_address.clone()),
    };

    let _res: Response = instantiate(&mut deps, env, info, instantiate_msg).unwrap();

    // Initialize the mixer
    let params = setup_params(Curve::Bn254, 5, 3);
    let poseidon = Poseidon::new(params);
    let res = poseidon.hash_two(&Fr::one(), &Fr::one()).unwrap();
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

    let response: Response = execute(&mut deps, mock_env(), info, deposit_cw20_msg).unwrap();
    assert_eq!(
        response.attributes,
        vec![attr("method", "deposit_cw20"), attr("result", "0")]
    );
}

// NOT: Didn't add the "withdraw" test since it needs the input params from "test_util" module.
