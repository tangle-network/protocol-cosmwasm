#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{attr, to_binary, StdError, Coin, Uint256, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
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
    let anchor = ANCHOR.load(deps.storage)?;
    
    // Validation 1. Check if the enough UST are sent.
    let sent_uusd: Vec<Coin> = info
        .funds
        .into_iter()
        .filter(|x| x.denom == "uusd")
        .collect();
    if sent_uusd.is_empty() || Uint256::from(sent_uusd[0].amount) < anchor.deposit_size {
        return Err(ContractError::InsufficientFunds {});
    }

    if let Some(commitment) = msg.commitment {
        let mut merkle_tree = anchor.merkle_tree;
        let poseidon = POSEIDON.load(deps.storage)?;
        let res = merkle_tree.insert(
            poseidon, 
            commitment, 
            deps.storage,
        ).map_err(|_| ContractError::MerkleTreeIsFull)?;

        ANCHOR.save(
            deps.storage, 
            &Anchor {
                chain_id: anchor.chain_id,
                deposit_size: anchor.deposit_size,
                linkable_tree: anchor.linkable_tree,
                merkle_tree: merkle_tree,
            }
        )?;

        Ok(Response::new().add_attributes(vec![
            attr("method", "deposit"),
            attr("result", res.to_string()),
        ]))
    } else {
        Err(ContractError::Std(StdError::NotFound {
            kind: "Commitment".to_string(),
        }))
    }
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
    use ark_bn254::Fr;
    use ark_crypto_primitives::CRH as CRHTrait;
    use ark_ff::PrimeField;
    use ark_ff::{BigInteger, Field};
    use ark_std::One;
    use arkworks_gadgets::merkle_tree::simple_merkle::gen_empty_hashes;
    use arkworks_gadgets::poseidon::CRH;
    use arkworks_utils::utils::bn254_x5_5::get_poseidon_bn254_x5_5;
    use arkworks_utils::utils::common::{Curve};
    use arkworks_utils::utils::parse_vec;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{attr, coins, Uint128};
    type PoseidonCRH5 = CRH<Fr>;
    use arkworks_gadgets::poseidon::field_hasher;

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

    #[test]
    fn test_deposit() {
        let mut deps = mock_dependencies(&coins(2, "token"));

        // Initialize the contract
        let env = mock_env();
        let info = mock_info("anyone", &[]);
        let instantiate_msg = InstantiateMsg {
            max_edges: 0,
            chain_id: 1,
            levels: 0,
            deposit_size: Uint128::from(1_000_000_u128),
        };

        let _ = instantiate(deps.as_mut(), env, info, instantiate_msg).unwrap();

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

        // Try the deposit with insufficient fund
        let info = mock_info("depositor", &[Coin::new(1_000_u128, "uusd")]);
        let deposit_msg = DepositMsg {
            from: None,
            commitment: Some(element),
            value: Uint256::from(0_u128),
        };

        let err = deposit(deps.as_mut(), info, deposit_msg).unwrap_err();
        assert_eq!(err.to_string(), "Insufficient_funds".to_string());

        // Try the deposit with empty commitment
        let info = mock_info("depositor", &[Coin::new(1_000_000_u128, "uusd")]);
        let deposit_msg = DepositMsg {
            from: None,
            commitment: None,
            value: Uint256::from(0_u128),
        };

        let err = deposit(deps.as_mut(), info, deposit_msg).unwrap_err();
        assert_eq!(err.to_string(), "Commitment not found".to_string());

        // Try the deposit for success
        let info = mock_info("depositor", &[Coin::new(1_000_000_u128, "uusd")]);
        let deposit_msg = DepositMsg {
            from: None,
            commitment: Some(element),
            value: Uint256::from(0_u128),
        };

        let response = deposit(deps.as_mut(), info, deposit_msg).unwrap();
        assert_eq!(
            response.attributes,
            vec![attr("method", "deposit"), attr("result", "0")]
        );
    }
}
