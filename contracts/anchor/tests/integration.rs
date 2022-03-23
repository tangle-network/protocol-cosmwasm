//! This integration test tries to run and call the generated wasm.

use cosmwasm_std::{attr, from_binary, to_binary, Response, Uint128};
use cosmwasm_vm::testing::{
    execute, instantiate, mock_env, mock_info, mock_instance_with_gas_limit, query,
};
use cw20::Cw20ReceiveMsg;
use protocol_cosmwasm::anchor::{Cw20HookMsg, ExecuteMsg, InfoResponse, InstantiateMsg, QueryMsg};

use ark_bn254::Fr;
use ark_ff::PrimeField;
use ark_ff::{BigInteger, Field};
use ark_std::One;
use arkworks_native_gadgets::poseidon::{FieldHasher, Poseidon};
use arkworks_setups::common::setup_params;
use arkworks_setups::Curve;

// This line will test the output of cargo wasm
// static WASM: &[u8] = include_bytes!("../../../artifacts/cosmwasm_anchor.wasm");

// You can uncomment this line instead to test productionified build from rust-optimizer
// static WASM: &[u8] =
//     include_bytes!("../../../target/wasm32-unknown-unknown/release/cosmwasm_anchor.wasm");

// For the github CI, we copy the wasm file manually & import here.
static WASM: &[u8] = include_bytes!("./cosmwasm_anchor.wasm");

const MAX_EDGES: u32 = 2;
const CHAIN_ID: u64 = 1;
const LEVELS: u32 = 30;
const CW20_ADDRESS: &str = "terra1fex9f78reuwhfsnc8sun6mz8rl9zwqh03fhwf3";
const DEPOSIT_SIZE: u128 = 1_000_000;
const DEPOSITOR: &str = "depositor";

#[test]
fn integration_test_instantiate_anchor() {
    // "Gas_limit" should be set high manually, since the low value can cause "GasDepletion" error.
    let mut deps = mock_instance_with_gas_limit(WASM, 100_000_000);

    let msg = InstantiateMsg {
        max_edges: MAX_EDGES,
        chain_id: CHAIN_ID,
        levels: LEVELS,
        deposit_size: Uint128::from(DEPOSIT_SIZE),
        cw20_address: CW20_ADDRESS.to_string(),
    };

    let info = mock_info("anyone", &[]);
    let response: Response = instantiate(&mut deps, mock_env(), info, msg).unwrap();

    assert_eq!(
        response.attributes,
        vec![attr("method", "instantiate"), attr("owner", "anyone"),]
    );

    let query = query(&mut deps, mock_env(), QueryMsg::GetCw20Address {}).unwrap();
    let info: InfoResponse = from_binary(&query).unwrap();
    assert_eq!(info.cw20_address, CW20_ADDRESS.to_string());
}

#[test]
fn test_deposit_cw20() {
    let mut deps = mock_instance_with_gas_limit(WASM, 200_000_000);

    // Initialize the contract
    let env = mock_env();
    let info = mock_info("anyone", &[]);
    let instantiate_msg = InstantiateMsg {
        max_edges: MAX_EDGES,
        chain_id: CHAIN_ID,
        levels: LEVELS,
        deposit_size: Uint128::from(DEPOSIT_SIZE),
        cw20_address: CW20_ADDRESS.to_string(),
    };

    let _res: Response = instantiate(&mut deps, env, info, instantiate_msg).unwrap();

    // Initialize the mixer
    let params = setup_params(Curve::Bn254, 5, 4);
    let poseidon = Poseidon::new(params);
    let res = poseidon.hash_two(&Fr::one(), &Fr::one()).unwrap();
    let mut element: [u8; 32] = [0u8; 32];
    element.copy_from_slice(&res.into_repr().to_bytes_le());

    // Should "deposit" cw20 tokens with success.
    let info = mock_info(CW20_ADDRESS, &[]);
    let deposit_cw20_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: CW20_ADDRESS.to_string(),
        amount: Uint128::from(DEPOSIT_SIZE),
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
