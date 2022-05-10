//! This integration test tries to run and call the generated wasm.

use cosmwasm_std::{attr, Response, Uint128};
use cosmwasm_vm::testing::{instantiate, mock_env, mock_info, mock_instance_with_gas_limit};

use protocol_cosmwasm::vanchor::InstantiateMsg;

// This line will test the output of cargo wasm
// static WASM: &[u8] = include_bytes!("../../../artifacts/cosmwasm_vanchor.wasm");

// You can uncomment this line instead to test productionified build from rust-optimizer
// static WASM: &[u8] =
//     include_bytes!("../../../target/wasm32-unknown-unknown/release/cosmwasm_vanchor.wasm");

// For the github CI, we copy the wasm file manually & import here.
static WASM: &[u8] = include_bytes!("./cosmwasm_vanchor.wasm");

const CHAIN_ID: u64 = 1;
const MAX_EDGES: u32 = 2;
const LEVELS: u32 = 30;
const MAX_DEPOSIT_AMT: u128 = 40;
const MIN_WITHDRAW_AMT: u128 = 0;
const MAX_EXT_AMT: u128 = 20;
const MAX_FEE: u128 = 10;
const CW20_ADDRESS: &str = "terra1340t6lqq6jxhm8d6gtz0hzz5jzcszvm27urkn2";
const HANDLER: &str = "terra1fex9f78reuwhfsnc8sun6mz8rl9zwqh03fhwf3";

#[test]
fn integration_test_instantiate_mixer() {
    // "Gas_limit" should be set high manually, since the low value can cause "GasDepletion" error.
    let mut deps = mock_instance_with_gas_limit(WASM, 100_000_000);

    let msg = InstantiateMsg {
        chain_id: CHAIN_ID,
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
    let response: Response = instantiate(&mut deps, mock_env(), info, msg).unwrap();

    assert_eq!(
        response.attributes,
        vec![attr("method", "instantiate"), attr("owner", "creator"),]
    );
}
