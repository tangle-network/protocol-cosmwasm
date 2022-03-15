#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, attr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::set_contract_version;

use protocol_cosmwasm::vanchor::{InstantiateMsg, ExecuteMsg, QueryMsg, UpdateConfigMsg};
use protocol_cosmwasm::error::ContractError;
use protocol_cosmwasm::vanchor_verifier::VAnchorVerifier;
use protocol_cosmwasm::poseidon::Poseidon;
use protocol_cosmwasm::zeroes::zeroes;

use crate::state::{
    save_root, save_subtree, VAnchor, LinkableMerkleTree, MerkleTree, VANCHOR, VANCHORVERIFIER,
    NULLIFIERS, POSEIDON
};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cosmwasm-vanchor";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    // Validation 1. Check if any funds are sent with this message.
    if !info.funds.is_empty() {
        return Err(ContractError::UnnecessaryFunds {  });
    }

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Initialize the poseidon hasher
    POSEIDON.save(deps.storage, &Poseidon::new())?;

    // Initialize the VAnchor_verifier
    VANCHORVERIFIER.save(deps.storage, &VAnchorVerifier::new())?;
  
    // Initialize the merkle tree
    let merkle_tree: MerkleTree = MerkleTree {
        levels: msg.levels,
        current_root_index: 0,
        next_index: 0,
    };

    // Initialize the linkable merkle tree
    let linkable_merkle_tree = LinkableMerkleTree {
        max_edges: msg.max_edges,
        chain_id_list: Vec::new(),
    };
    // Get the "cw20_address"
    let cw20_address = deps.api.addr_canonicalize(&msg.cw20_address)?;

    // Initialize the VAnchor
    let anchor = VAnchor {
        creator: deps.api.addr_canonicalize(info.sender.as_str())?,
        max_deposit_amt: msg.max_deposit_amt,
        min_withdraw_amt: msg.min_withdraw_amt,
        max_ext_amt: msg.max_ext_amt,
        max_fee: msg.max_fee,
        linkable_tree: linkable_merkle_tree,
        merkle_tree,
        cw20_address,
    };
    VANCHOR.save(deps.storage, &anchor)?;

    // Initialize the "FILLED_SUBTREES" with "zero" data.
    for i in 0..msg.levels {
        save_subtree(deps.storage, i as u32, &zeroes(i))?;
    }

    // Initialize the (merkletree) "ROOTS" with "zero" data.
    save_root(deps.storage, 0_u32, &zeroes(msg.levels))?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateConfig(msg) => update_vanchor_config(deps, info, msg),
    }
}

fn update_vanchor_config(deps: DepsMut, info: MessageInfo, msg: UpdateConfigMsg) -> Result<Response, ContractError> {
    // Validation 1. Check if any funds are sent with this message.
    if !info.funds.is_empty() {
        return Err(ContractError::UnnecessaryFunds {  });
    }

    let mut vanchor = VANCHOR.load(deps.storage)?;
    // Validation 2. Check if the msg sender is "creator".
    if vanchor.creator != deps.api.addr_canonicalize(info.sender.as_str())? {
        return Err(ContractError::Unauthorized {  });
    }

    // Update the vanchor config.
    if let Some(max_deposit_amt) = msg.max_deposit_amt {
        vanchor.max_deposit_amt = max_deposit_amt;
    }

    if let Some(min_withdraw_amt) = msg.min_withdraw_amt {
        vanchor.min_withdraw_amt = min_withdraw_amt;
    }

    if let Some(max_ext_amt) = msg.max_ext_amt {
        vanchor.max_ext_amt = max_ext_amt;
    }

    if let Some(max_fee) = msg.max_fee {
        vanchor.max_fee = max_fee;
    }

    VANCHOR.save(deps.storage, &vanchor)?;

    Ok(Response::new().add_attributes(vec![
        attr("method", "update_vanchor_config"),
    ]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        // TODO
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coins, from_binary, Uint256};

    #[test]
    fn proper_initialization() {
        let cw20_address = "terra1fex9f78reuwhfsnc8sun6mz8rl9zwqh03fhwf3".to_string();
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg {
            max_edges: 0,
            levels: 0,
            max_deposit_amt: Uint256::zero(),
            min_withdraw_amt: Uint256::zero(),
            max_ext_amt: Uint256::zero(),
            max_fee: Uint256::zero(),
            cw20_address: cw20_address,
        };
        let info = mock_info("creator", &[]);

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn test_vanchor_update_config() {
        let cw20_address = "terra1fex9f78reuwhfsnc8sun6mz8rl9zwqh03fhwf3".to_string();
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg {
            max_edges: 0,
            levels: 0,
            max_deposit_amt: Uint256::zero(),
            min_withdraw_amt: Uint256::zero(),
            max_ext_amt: Uint256::zero(),
            max_fee: Uint256::zero(),
            cw20_address: cw20_address,
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
            execute(deps.as_mut(), mock_env(), info, ExecuteMsg::UpdateConfig(update_config_msg)).is_err(),
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
        let _ = execute(deps.as_mut(), mock_env(), info, ExecuteMsg::UpdateConfig(update_config_msg)).unwrap();
    }
}
