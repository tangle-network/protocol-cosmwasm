#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{attr, to_binary, Uint256, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::set_contract_version;

use protocol_cosmwasm::zeroes::zeroes;
use protocol_cosmwasm::anchor_verifier::AnchorVerifier;
use protocol_cosmwasm::poseidon::Poseidon;
use protocol_cosmwasm::error::ContractError;
use protocol_cosmwasm::anchor::{ExecuteMsg, InstantiateMsg, QueryMsg, DepositMsg, WithdrawMsg};

use crate::state::{POSEIDON, ANCHORVERIFIER, save_subtree, save_root, MerkleTree, LinkableMerkleTree, Anchor, ANCHOR};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cosmwasm-anchor";
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
        return Err(ContractError::UnnecessaryFunds {});
    }
    
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Initialize the poseidon hasher
    POSEIDON.save(deps.storage, &Poseidon::new())?;

    // Initialize the Anchor_verifier
    ANCHORVERIFIER.save(deps.storage, &AnchorVerifier::new())?;

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

    // Initialize the Anchor
    let anchor = Anchor {
        chain_id: msg.chain_id,
        deposit_size: Uint256::from(msg.deposit_size.u128()),
        merkle_tree: merkle_tree,
        linkable_tree: linkable_merkle_tree,
    };
    ANCHOR.save(deps.storage, &anchor)?;

    for i in 0..msg.levels {
        save_subtree(deps.storage, i as u32, &zeroes(i))?;
    }

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
        ExecuteMsg::Deposit(msg) => deposit(deps, info, msg),
        ExecuteMsg::Withdraw(msg) => withdraw(deps, info, msg),
    }
}

pub fn deposit(
    deps: DepsMut,
    info: MessageInfo,
    msg: DepositMsg
) -> Result<Response, ContractError> {
    // TODO
    Ok(Response::new().add_attributes(vec![
        attr("method", "deposit")
    ]))
}

pub fn withdraw(
    deps: DepsMut,
    info: MessageInfo,
    msg: WithdrawMsg,
) -> Result<Response, ContractError> {
    // TODO
    Ok(Response::new().add_attributes(vec![
        attr("method", "withdraw")
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
    use cosmwasm_std::{coins, from_binary, Uint128};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(&[]);

        let env = mock_env();
        let info = mock_info("anyone", &[]);
        let instantiate_msg = InstantiateMsg {
            max_edges: 0,
            chain_id: 1,
            levels: 0,
            deposit_size: Uint128::from(1_000_000_u128),
        };

         // Should pass this "unwrap" if success.
         let response = instantiate(deps.as_mut(), env, info, instantiate_msg).unwrap();

         assert_eq!(
             response.attributes,
             vec![attr("method", "instantiate"), attr("owner", "anyone"),]
         );
    }
}
