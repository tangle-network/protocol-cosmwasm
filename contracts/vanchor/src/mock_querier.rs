// Contains mock functionality to test multi-contract scenarios

use std::marker::PhantomData;

use cosmwasm_std::testing::{MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    from_binary, from_slice, to_binary, Coin, ContractResult, Empty, OwnedDeps, Querier,
    QuerierResult, QueryRequest, SystemError, SystemResult, WasmQuery, Uint128,
};
use protocol_cosmwasm::utils::parse_string_to_uint128;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use protocol_cosmwasm::token_wrapper::{
    ConfigResponse as TokenWrapperConfigResponse, GetAmountToWrapResponse,
};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
    GetAmountToWrap { target_amount: String },
}

/// mock_dependencies is a drop-in replacement for cosmwasm_std::testing::mock_dependencies
/// this uses CustomQuerier.
pub fn mock_dependencies(
    contract_balance: &[Coin],
) -> OwnedDeps<MockStorage, MockApi, WasmMockQuerier> {
    let custom_querier: WasmMockQuerier =
        WasmMockQuerier::new(MockQuerier::new(&[(&MOCK_CONTRACT_ADDR, contract_balance)]));

    OwnedDeps {
        storage: MockStorage::default(),
        api: MockApi::default(),
        querier: custom_querier,
        custom_query_type: PhantomData,
    }
}

pub struct WasmMockQuerier {
    base: MockQuerier,
}

impl Querier for WasmMockQuerier {
    fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
        // MockQuerier doesn't support Custom, so we ignore it completely here
        let request: QueryRequest<Empty> = match from_slice(bin_request) {
            Ok(v) => v,
            Err(e) => {
                return SystemResult::Err(SystemError::InvalidRequest {
                    error: format!("Parsing query request: {}", e),
                    request: bin_request.into(),
                })
            }
        };
        self.handle_query(&request)
    }
}

impl WasmMockQuerier {
    pub fn new(base: MockQuerier) -> Self {
        WasmMockQuerier { base }
    }

    pub fn handle_query(&self, request: &QueryRequest<Empty>) -> QuerierResult {
        match &request {
            QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: _,
                msg,
            }) => match from_binary(&msg).unwrap() {
                QueryMsg::Config {} => SystemResult::Ok(ContractResult::Ok(
                    to_binary(&TokenWrapperConfigResponse {
                        governor: "governor".to_string(),
                        native_token_denom: "uusd".to_string(),
                        fee_recipient: "fee-recipient".to_string(),
                        fee_percentage: "0.01".to_string(),
                        is_native_allowed: "true".to_string(),
                        wrapping_limit: "100".to_string(),
                        proposal_nonce: "0".to_string(),
                    })
                    .unwrap(),
                )),
                QueryMsg::GetAmountToWrap { target_amount } => {
                    let targ_amt = parse_string_to_uint128(target_amount).unwrap();
                    // Assumes that the "fee_percentage" is 10%
                    let amt_to_wrap = targ_amt.multiply_ratio(100_u128, 90_u128);
                    SystemResult::Ok(ContractResult::Ok(
                        to_binary(&GetAmountToWrapResponse {
                            target_amount: targ_amt.to_string(),
                            amount_to_wrap: amt_to_wrap.to_string(),
                        })
                        .unwrap(),
                    ))
                }
            },
            _ => self.base.handle_query(request),
        }
    }
}
