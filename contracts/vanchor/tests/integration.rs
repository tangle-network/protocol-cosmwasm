//! This integration test tries to run and call the generated wasm.

use cosmwasm_std::{attr, Response, Uint256};
use cosmwasm_vm::testing::{instantiate, mock_env, mock_info, mock_instance_with_gas_limit};

use protocol_cosmwasm::vanchor::InstantiateMsg;

// This line will test the output of cargo wasm
// static WASM: &[u8] = include_bytes!("../../../artifacts/cosmwasm_vanchor.wasm");

// You can uncomment this line instead to test productionified build from rust-optimizer
// static WASM: &[u8] =
//     include_bytes!("../../../target/wasm32-unknown-unknown/release/cosmwasm_vanchor.wasm");

// For the github CI, we copy the wasm file manually & import here.
static WASM: &[u8] = include_bytes!("./cosmwasm_vanchor.wasm");

#[test]
fn integration_test_instantiate_mixer() {
    // "Gas_limit" should be set high manually, since the low value can cause "GasDepletion" error.
    let mut deps = mock_instance_with_gas_limit(WASM, 100_000_000);

    let cw20_address = "terra1fex9f78reuwhfsnc8sun6mz8rl9zwqh03fhwf3".to_string();

    let msg = InstantiateMsg {
        chain_id: 1,
        max_edges: 2,
        levels: 30,
        max_deposit_amt: Uint256::from(40_u128),
        min_withdraw_amt: Uint256::from(0_u128),
        max_ext_amt: Uint256::from(20_u128),
        max_fee: Uint256::from(10_u128),
        cw20_address: cw20_address.clone(),
    };
    let info = mock_info("creator", &[]);
    let response: Response = instantiate(&mut deps, mock_env(), info, msg).unwrap();

    assert_eq!(
        response.attributes,
        vec![attr("method", "instantiate"), attr("owner", "creator"),]
    );
}
