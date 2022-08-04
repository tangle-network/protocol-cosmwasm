use std::str::FromStr;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, from_binary, to_binary, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, Event,
    MessageInfo, Response, StdError, StdResult, Storage, Uint128, WasmMsg,
};
use cw2::set_contract_version;

use crate::state::{
    read_curr_neighbor_root_index, read_edge, read_neighbor_roots, read_root,
    save_curr_neighbor_root_index, save_edge, save_neighbor_roots, save_root, save_subtree, Anchor,
    LinkableMerkleTree, MerkleTree, ANCHOR, HASHER, NULLIFIERS, VERIFIER,
};
use codec::Encode;
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};
use protocol_cosmwasm::anchor::{
    ConfigResponse, Cw20HookMsg, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, WithdrawMsg,
};
use protocol_cosmwasm::anchor_verifier::AnchorVerifier;
use protocol_cosmwasm::error::ContractError;
use protocol_cosmwasm::keccak::Keccak256;
use protocol_cosmwasm::poseidon::Poseidon;
use protocol_cosmwasm::structs::{
    Edge, EdgeInfoResponse, MerkleRootInfoResponse, MerkleTreeInfoResponse,
    NeighborRootInfoResponse, COSMOS_CHAIN_TYPE, HISTORY_LENGTH,
};
use protocol_cosmwasm::token_wrapper::{
    ConfigResponse as TokenWrapperConfigResp, Cw20HookMsg as TokenWrapperHookMsg,
    ExecuteMsg as TokenWrapperExecuteMsg, GetAmountToWrapResponse,
    QueryMsg as TokenWrapperQueryMsg,
};
use protocol_cosmwasm::utils::{
    compute_chain_id, compute_chain_id_type, element_encoder, truncate_and_pad,
};
use protocol_cosmwasm::zeroes::zeroes;

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
    HASHER.save(deps.storage, &Poseidon::new())?;

    // Initialize the Anchor_verifier
    let anchor_verifier = match AnchorVerifier::new(msg.max_edges) {
        Ok(v) => v,
        Err(e) => return Err(ContractError::Std(e)),
    };
    VERIFIER.save(deps.storage, &anchor_verifier)?;

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

    // Get the "TokenWrapper" token address.
    let tokenwrapper_addr = deps.api.addr_validate(msg.tokenwrapper_addr.as_str())?;

    // Get the "handler" address
    let handler = deps.api.addr_validate(&msg.handler)?;

    // Initialize the Anchor
    let deposit_size = msg.deposit_size;
    let anchor = Anchor {
        linkable_tree: linkable_merkle_tree,
        proposal_nonce: 0_u32,
        deposit_size,
        merkle_tree,
        tokenwrapper_addr,
        handler,
    };
    ANCHOR.save(deps.storage, &anchor)?;

    // Initialize the "FILLED_SUBTREES" with "zero" data.
    for i in 0..msg.levels {
        save_subtree(deps.storage, i as u32, &zeroes(i))?;
    }

    // Initialize the (merkletree) "ROOTS" with "zero" data.
    save_root(deps.storage, 0_u32, &zeroes(msg.levels))?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("owner", info.sender))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        // Withdraw the cw20 token with proof
        ExecuteMsg::Withdraw(msg) => withdraw(deps, env, info, msg),
        // Unwrap the "TokenWrapper" token
        ExecuteMsg::UnwrapIntoToken { token_addr, amount } => {
            unwrap_into_token(deps, info.sender.to_string(), token_addr, amount)
        }
        // Handle "receive" cw20 token
        // 1. DepositCw20
        // 2. WrapToken
        ExecuteMsg::Receive(msg) => receive_cw20(deps, env, info, msg),
        // Wrap the native token
        ExecuteMsg::WrapNative { amount } => {
            wrap_native(deps, info.sender.to_string(), amount, info.funds)
        }
        // Unwrap the "TokenWrapper" token to native token
        ExecuteMsg::UnwrapNative { amount } => unwrap_native(deps, info.sender.to_string(), amount),

        // Wrap the native token & deposit it into the contract
        ExecuteMsg::WrapAndDeposit { commitment, amount } => wrap_and_deposit_native(
            deps,
            env.clone(),
            info.sender.to_string(),
            env.contract.address.to_string(),
            commitment,
            amount,
            info.funds,
        ),

        // Withdraws the deposit & unwraps to valid token for `sender`
        ExecuteMsg::WithdrawAndUnwrap(msg) => withdraw_and_unwrap(deps, env, info, msg),

        // Sets a new handler for the contract
        ExecuteMsg::SetHandler { handler, nonce } => set_handler(deps, info, handler, nonce),

        // Update/add an edge for underlying tree
        ExecuteMsg::UpdateEdge {
            src_chain_id,
            root,
            latest_leaf_index,
            target,
        } => {
            let linkable_tree = ANCHOR.load(deps.storage)?.linkable_tree;
            if linkable_tree.has_edge(src_chain_id, deps.storage) {
                update_edge(deps, src_chain_id, root, latest_leaf_index, target)
            } else {
                add_edge(deps, src_chain_id, root, latest_leaf_index, target)
            }
        }
    }
}

pub fn receive_cw20(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let recv_token_addr = info.sender.to_string();
    let recv_token_amt = cw20_msg.amount;
    let sender = cw20_msg.sender;
    match from_binary(&cw20_msg.msg) {
        Ok(Cw20HookMsg::DepositCw20 { commitment }) => {
            deposit_cw20(deps, env, commitment, recv_token_addr, recv_token_amt)
        }
        Ok(Cw20HookMsg::WrapToken {}) => wrap_token(deps, sender, recv_token_addr, recv_token_amt),
        Ok(Cw20HookMsg::WrapAndDeposit { commitment, amount }) => wrap_and_deposit_cw20(
            deps,
            env.clone(),
            sender,
            env.contract.address.to_string(),
            commitment,
            amount,
            recv_token_addr,
            recv_token_amt,
        ),
        Err(_) => Err(ContractError::InvalidCw20HookMsg {}),
    }
}

/// Deposit CW20 token("TokenWrapper") with the commitment
fn deposit_cw20(
    mut deps: DepsMut,
    env: Env,
    commitment: Option<[u8; 32]>,
    recv_token_addr: String,
    recv_token_amt: Uint128,
) -> Result<Response, ContractError> {
    let anchor: Anchor = ANCHOR.load(deps.storage)?;

    // Validations
    let cw20_addr = deps.api.addr_validate(recv_token_addr.as_str())?;
    if anchor.tokenwrapper_addr != cw20_addr {
        return Err(ContractError::Unauthorized {});
    }

    if recv_token_amt < anchor.deposit_size {
        return Err(ContractError::InsufficientFunds {});
    }

    // Handle the "deposit" cw20 tokens
    if let Some(commitment) = commitment {
        // Handle the "commitment"
        let inserted_index = validate_and_store_commitment(deps.branch(), commitment)?;

        // No need to handle any cw20 token transfer
        // since "TokenWrapper" tokens are already sent to this contract
        Ok(
            Response::new().add_event(Event::new("anchor-deposit").add_attributes(vec![
                attr("action", "deposit_cw20"),
                attr("inserted_index", inserted_index.to_string()),
                attr("commitment", format!("{:?}", commitment)),
                attr("timestamp", env.block.time.seconds().to_string()),
            ])),
        )
    } else {
        Err(ContractError::Std(StdError::NotFound {
            kind: "Commitment".to_string(),
        }))
    }
}

/// Wrap the cw20 token into "TokenWrapper" token
fn wrap_token(
    deps: DepsMut,
    sender: String,
    recv_token_addr: String,
    recv_token_amt: Uint128,
) -> Result<Response, ContractError> {
    let anchor = ANCHOR.load(deps.storage)?;

    // Validations
    let cw20_addr = deps.api.addr_validate(recv_token_addr.as_str())?;
    if anchor.tokenwrapper_addr == cw20_addr {
        return Err(ContractError::Unauthorized {});
    }

    // Handle the "Wrap" function
    let msgs: Vec<CosmosMsg> = vec![CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: recv_token_addr.clone(),
        funds: vec![],
        msg: to_binary(&Cw20ExecuteMsg::Send {
            contract: anchor.tokenwrapper_addr.to_string(),
            amount: recv_token_amt,
            msg: to_binary(&TokenWrapperHookMsg::Wrap {
                sender: Some(sender.clone()),
                recipient: Some(sender),
            })?,
        })?,
    })];

    Ok(Response::new().add_messages(msgs).add_attributes(vec![
        attr("action", "wrap_token"),
        attr("token", recv_token_addr),
        attr("amount", recv_token_amt),
    ]))
}

/// Unwrap the "TokenWrapper" token into "token"
fn unwrap_into_token(
    deps: DepsMut,
    sender: String,
    token_addr: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let anchor = ANCHOR.load(deps.storage)?;

    // Handle the "Unwrap"
    let msgs: Vec<CosmosMsg> = vec![CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: anchor.tokenwrapper_addr.to_string(),
        funds: vec![],
        msg: to_binary(&TokenWrapperExecuteMsg::Unwrap {
            sender: Some(sender.clone()),
            recipient: Some(sender),
            token: Some(deps.api.addr_validate(token_addr.as_str())?),
            amount,
        })?,
    })];

    Ok(Response::new().add_messages(msgs).add_attributes(vec![
        attr("action", "unwrap_into_token"),
        attr("token", token_addr),
        attr("amount", amount),
    ]))
}

/// Withdraw a deposit from the contract
pub fn withdraw(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: WithdrawMsg,
) -> Result<Response, ContractError> {
    let recipient = msg.recipient.clone();
    let relayer = msg.relayer.clone();
    let fee = msg.fee;
    let refund = msg.refund;
    let sent_funds = info.funds;
    if !refund.is_zero() && (sent_funds.len() != 1 || sent_funds[0].amount != refund) {
        return Err(ContractError::InsufficientFunds {});
    }

    let anchor = ANCHOR.load(deps.storage)?;

    // Validation 1. Check if the root is known to merkle tree.
    let merkle_tree = anchor.merkle_tree;
    if !merkle_tree.is_known_root(msg.roots[0], deps.storage) {
        return Err(ContractError::UnknownRoot {});
    }

    // Validation 2. Check if the roots are valid in linkable tree.
    let linkable_tree = anchor.linkable_tree;
    if !linkable_tree.is_valid_neighbor_roots(&msg.roots[1..], deps.storage) {
        return Err(ContractError::InvaidMerkleRoots {});
    }

    // Validation 3. Check if the nullifier already used.
    if is_known_nullifier(deps.storage, msg.nullifier_hash) {
        return Err(ContractError::AlreadyRevealedNullfier {});
    }

    // Format the public input bytes
    let chain_id = compute_chain_id(&env.block.chain_id);
    let chain_id_type_bytes =
        element_encoder(&compute_chain_id_type(chain_id.into(), &COSMOS_CHAIN_TYPE).to_le_bytes());
    let recipient_bytes = truncate_and_pad(recipient.as_bytes());
    let relayer_bytes = truncate_and_pad(relayer.as_bytes());

    let mut arbitrary_data_bytes = Vec::new();
    arbitrary_data_bytes.extend_from_slice(&recipient_bytes);
    arbitrary_data_bytes.extend_from_slice(&relayer_bytes);
    arbitrary_data_bytes.extend_from_slice(&fee.u128().encode());
    arbitrary_data_bytes.extend_from_slice(&refund.u128().encode());
    arbitrary_data_bytes.extend_from_slice(&msg.commitment);
    let arbitrary_input =
        Keccak256::hash(&arbitrary_data_bytes).map_err(|_| ContractError::HashError)?;

    // Join the public input bytes
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&msg.nullifier_hash);
    bytes.extend_from_slice(&arbitrary_input);
    bytes.extend_from_slice(&chain_id_type_bytes);
    for root in msg.roots {
        bytes.extend_from_slice(&root);
    }

    // Verify the proof
    let verifier = VERIFIER.load(deps.storage)?;
    let result = verify(verifier, bytes, msg.proof_bytes)?;

    if !result {
        return Err(ContractError::InvalidWithdrawProof {});
    }

    // Set used nullifier to true after successful verification
    NULLIFIERS.save(deps.storage, msg.nullifier_hash.to_vec(), &true)?;

    // Validate the "cw20_address".
    let cw20_address = msg
        .cw20_address
        .expect("Token address should be given for the withdrawal");
    if anchor.tokenwrapper_addr != deps.api.addr_validate(cw20_address.as_str())? {
        return Err(ContractError::InvalidCw20Token);
    }

    // Send the funds
    let mut msgs: Vec<CosmosMsg> = vec![];

    // Send the funds to "recipient"
    let amt_to_recipient = match anchor.deposit_size.checked_sub(fee) {
        Ok(v) => v,
        Err(e) => {
            return Err(ContractError::Std(StdError::GenericErr {
                msg: e.to_string(),
            }))
        }
    };

    if !amt_to_recipient.is_zero() {
        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cw20_address.clone(),
            funds: [].to_vec(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: recipient.clone(),
                amount: amt_to_recipient,
            })?,
        }));
    }

    // Send the funds to "relayer"
    if !fee.is_zero() {
        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cw20_address,
            funds: [].to_vec(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: relayer,
                amount: fee,
            })?,
        }));
    }

    // If "refund" field is non-zero, send the funds to "recipient"
    if !refund.is_zero() {
        msgs.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: recipient,
            amount: sent_funds,
        }));
    }

    Ok(Response::new()
        .add_messages(msgs)
        .add_event(Event::new("anchor-withdraw").add_attributes(vec![
            attr("action", "withdraw"),
            attr("recipient", msg.recipient),
            attr("relayer", msg.relayer),
            attr("fee", msg.fee),
            attr("commitment", format!("{:?}", msg.commitment)),
            attr("nullifier_hash", format!("{:?}", msg.nullifier_hash)),
        ])))
}

/// Wrap the native token into "TokenWrapper" token
fn wrap_native(
    deps: DepsMut,
    sender: String,
    amount: Uint128,
    sent_funds: Vec<Coin>,
) -> Result<Response, ContractError> {
    let anchor = ANCHOR.load(deps.storage)?;

    // Validations
    let wrapper_config: TokenWrapperConfigResp = deps.querier.query_wasm_smart(
        anchor.tokenwrapper_addr.to_string(),
        &TokenWrapperQueryMsg::Config {},
    )?;
    let token_denom = wrapper_config.native_token_denom;

    let is_sent_enough_token = sent_funds
        .iter()
        .any(|c| c.denom == token_denom.clone() && c.amount == amount);
    if !is_sent_enough_token {
        return Err(ContractError::InsufficientFunds {});
    }

    // Handle the "wrap"
    let msgs: Vec<CosmosMsg> = vec![CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: anchor.tokenwrapper_addr.to_string(),
        funds: sent_funds,
        msg: to_binary(&TokenWrapperExecuteMsg::Wrap {
            sender: Some(sender.clone()),
            recipient: Some(sender),
        })?,
    })];

    Ok(Response::new().add_messages(msgs).add_attributes(vec![
        attr("action", "wrap_native"),
        attr("denom", token_denom),
        attr("amount", amount),
    ]))
}

/// Unwrap the "TokenWrapper" token into "token"
fn unwrap_native(
    deps: DepsMut,
    sender: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let anchor = ANCHOR.load(deps.storage)?;

    // Handle the "Unwrap"
    let msgs: Vec<CosmosMsg> = vec![CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: anchor.tokenwrapper_addr.to_string(),
        funds: vec![],
        msg: to_binary(&TokenWrapperExecuteMsg::Unwrap {
            sender: Some(sender.clone()),
            recipient: Some(sender),
            token: None,
            amount,
        })?,
    })];

    Ok(Response::new().add_messages(msgs).add_attributes(vec![
        attr("action", "unwrap_native"),
        attr("amount", amount),
    ]))
}

/// Wrap the native token & deposit it into the contract
fn wrap_and_deposit_native(
    mut deps: DepsMut,
    env: Env,
    sender: String,
    recipient: String,
    commitment: Option<[u8; 32]>,
    amount: Uint128,
    sent_funds: Vec<Coin>,
) -> Result<Response, ContractError> {
    let anchor = ANCHOR.load(deps.storage)?;
    let tokenwrapper = anchor.tokenwrapper_addr.as_str();

    // Validations
    let wrapper_config: TokenWrapperConfigResp = deps.querier.query_wasm_smart(
        anchor.tokenwrapper_addr.to_string(),
        &TokenWrapperQueryMsg::Config {},
    )?;
    let token_denom = wrapper_config.native_token_denom;

    let amt_to_wrap_query: GetAmountToWrapResponse = deps.querier.query_wasm_smart(
        tokenwrapper.to_string(),
        &TokenWrapperQueryMsg::GetAmountToWrap {
            target_amount: amount.to_string(),
        },
    )?;
    let amt_to_wrap = Uint128::from_str(&amt_to_wrap_query.amount_to_wrap)?;

    let is_sent_enough_token = sent_funds
        .iter()
        .any(|c| c.denom == token_denom.clone() && c.amount == amt_to_wrap);
    if !is_sent_enough_token {
        return Err(ContractError::InsufficientFunds {});
    }

    // Handle the "deposit"
    if let Some(commitment) = commitment {
        // Handle the "commitment"
        let inserted_index = validate_and_store_commitment(deps.branch(), commitment)?;

        // Wrap into the token and send directly to this contract
        let msgs: Vec<CosmosMsg> = vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: tokenwrapper.to_string(),
            msg: to_binary(&TokenWrapperExecuteMsg::Wrap {
                sender: Some(sender),
                recipient: Some(recipient),
            })?,
            funds: sent_funds,
        })];

        Ok(Response::new().add_messages(msgs).add_event(
            Event::new("anchor-deposit").add_attributes(vec![
                attr("action", "wrap_and_deposit_native"),
                attr("inserted_index", inserted_index.to_string()),
                attr("commitment", format!("{:?}", commitment)),
                attr("timestamp", env.block.time.seconds().to_string()),
            ]),
        ))
    } else {
        Err(ContractError::Std(StdError::NotFound {
            kind: "Commitment".to_string(),
        }))
    }
}

/// Wrap the cw20 token & deposit it into the contract
#[allow(clippy::too_many_arguments)]
fn wrap_and_deposit_cw20(
    mut deps: DepsMut,
    env: Env,
    sender: String,
    recipient: String,
    commitment: Option<[u8; 32]>,
    amount: Uint128,
    recv_token_addr: String,
    recv_token_amt: Uint128,
) -> Result<Response, ContractError> {
    let anchor = ANCHOR.load(deps.storage)?;
    let tokenwrapper = anchor.tokenwrapper_addr.as_str();

    // Only non-"TokenWrapper" Cw20 token contract can execute this message.
    if anchor.tokenwrapper_addr == deps.api.addr_validate(recv_token_addr.as_str())? {
        return Err(ContractError::Unauthorized {});
    }

    // Check if the "recv_token_amt" == "ext_amt" + "wrapping_fee"
    let amt_to_wrap_query: GetAmountToWrapResponse = deps.querier.query_wasm_smart(
        tokenwrapper.to_string(),
        &TokenWrapperQueryMsg::GetAmountToWrap {
            target_amount: amount.to_string(),
        },
    )?;
    let amt_to_wrap = Uint128::from_str(&amt_to_wrap_query.amount_to_wrap)?;

    if recv_token_amt != amt_to_wrap {
        return Err(ContractError::InsufficientFunds {});
    }

    // Handle the "deposit"
    if let Some(commitment) = commitment {
        // Handle the "commitment"
        let inserted_index = validate_and_store_commitment(deps.branch(), commitment)?;

        // Wrap into the token and send directly to this contract
        let msgs: Vec<CosmosMsg> = vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: recv_token_addr,
            msg: to_binary(&Cw20ExecuteMsg::Send {
                contract: tokenwrapper.to_string(),
                amount: amt_to_wrap,
                msg: to_binary(&TokenWrapperHookMsg::Wrap {
                    sender: Some(sender),
                    recipient: Some(recipient),
                })?,
            })?,
            funds: vec![],
        })];

        Ok(Response::new().add_messages(msgs).add_event(
            Event::new("anchor-deposit").add_attributes(vec![
                attr("action", "wrap_and_deposit_cw20"),
                attr("inserted_index", inserted_index.to_string()),
                attr("commitment", format!("{:?}", commitment)),
                attr("timestamp", env.block.time.seconds().to_string()),
            ]),
        ))
    } else {
        Err(ContractError::Std(StdError::NotFound {
            kind: "Commitment".to_string(),
        }))
    }
}

/// Withdraws the deposit & unwraps into valid token for `sender`
fn withdraw_and_unwrap(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: WithdrawMsg,
) -> Result<Response, ContractError> {
    let recipient = msg.recipient.clone();
    let relayer = msg.relayer.clone();
    let fee = msg.fee;
    let refund = msg.refund;
    let sent_funds = info.funds;
    if !refund.is_zero() && (sent_funds.len() != 1 || sent_funds[0].amount != refund) {
        return Err(ContractError::InsufficientFunds {});
    }

    let anchor = ANCHOR.load(deps.storage)?;

    // Validation 1. Check if the root is known to merkle tree.
    let merkle_tree = anchor.merkle_tree;
    if !merkle_tree.is_known_root(msg.roots[0], deps.storage) {
        return Err(ContractError::UnknownRoot {});
    }

    // Validation 2. Check if the roots are valid in linkable tree.
    let linkable_tree = anchor.linkable_tree;
    if !linkable_tree.is_valid_neighbor_roots(&msg.roots[1..], deps.storage) {
        return Err(ContractError::Std(StdError::GenericErr {
            msg: "Neighbor roots are not valid".to_string(),
        }));
    }

    // Validation 3. Check if the nullifier already used.
    if is_known_nullifier(deps.storage, msg.nullifier_hash) {
        return Err(ContractError::AlreadyRevealedNullfier {});
    }

    //
    let element_encoder = |v: &[u8]| {
        let mut output = [0u8; 32];
        output.iter_mut().zip(v).for_each(|(b1, b2)| *b1 = *b2);
        output
    };

    // Format the public input bytes
    let chain_id = compute_chain_id(&env.block.chain_id);
    let chain_id_type_bytes =
        element_encoder(&compute_chain_id_type(chain_id.into(), &COSMOS_CHAIN_TYPE).to_le_bytes());
    let recipient_bytes = truncate_and_pad(recipient.as_bytes());
    let relayer_bytes = truncate_and_pad(relayer.as_bytes());

    let mut arbitrary_data_bytes = Vec::new();
    arbitrary_data_bytes.extend_from_slice(&recipient_bytes);
    arbitrary_data_bytes.extend_from_slice(&relayer_bytes);
    arbitrary_data_bytes.extend_from_slice(&fee.u128().encode());
    arbitrary_data_bytes.extend_from_slice(&refund.u128().encode());
    arbitrary_data_bytes.extend_from_slice(&msg.commitment);
    let arbitrary_input =
        Keccak256::hash(&arbitrary_data_bytes).map_err(|_| ContractError::HashError)?;

    // Join the public input bytes
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&msg.nullifier_hash);
    bytes.extend_from_slice(&arbitrary_input);
    bytes.extend_from_slice(&chain_id_type_bytes);
    for root in msg.roots {
        bytes.extend_from_slice(&root);
    }

    // Verify the proof
    let verifier = VERIFIER.load(deps.storage)?;
    let result = verify(verifier, bytes, msg.proof_bytes)?;

    if !result {
        return Err(ContractError::InvalidWithdrawProof {});
    }

    // Set used nullifier to true after successful verification
    NULLIFIERS.save(deps.storage, msg.nullifier_hash.to_vec(), &true)?;

    // Send the funds
    let mut msgs: Vec<CosmosMsg> = vec![];

    // Send the funds to "recipient"
    let amt_to_recipient = match anchor.deposit_size.checked_sub(fee) {
        Ok(v) => v,
        Err(e) => {
            return Err(ContractError::Std(StdError::GenericErr {
                msg: e.to_string(),
            }))
        }
    };

    if !amt_to_recipient.is_zero() {
        let token_address = msg
            .cw20_address
            .map(|v| deps.api.addr_validate(v.as_str()).unwrap());
        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: anchor.tokenwrapper_addr.to_string(),
            funds: [].to_vec(),
            msg: to_binary(&TokenWrapperExecuteMsg::Unwrap {
                sender: None,
                token: token_address,
                amount: amt_to_recipient,
                recipient: Some(recipient.clone()),
            })?,
        }));
    }

    // Send the funds to "relayer"
    if !fee.is_zero() {
        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: anchor.tokenwrapper_addr.to_string(),
            funds: [].to_vec(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: relayer,
                amount: fee,
            })?,
        }));
    }

    // If "refund" field is non-zero, send the funds to "recipient"
    if !refund.is_zero() {
        msgs.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: recipient,
            amount: sent_funds,
        }));
    }

    Ok(Response::new()
        .add_messages(msgs)
        .add_event(Event::new("anchor-withdraw").add_attributes(vec![
            attr("action", "withdraw_and_unwrap"),
            attr("recipient", msg.recipient),
            attr("relayer", msg.relayer),
            attr("fee", msg.fee),
            attr("commitment", format!("{:?}", msg.commitment)),
            attr("nullifier_hash", format!("{:?}", msg.nullifier_hash)),
        ])))
}

/// Sets a new handler for the contract
fn set_handler(
    deps: DepsMut,
    info: MessageInfo,
    handler: String,
    nonce: u32,
) -> Result<Response, ContractError> {
    let mut anchor = ANCHOR.load(deps.storage)?;
    let curr_handler = anchor.handler;
    let proposal_nonce = anchor.proposal_nonce;

    // Validations
    if info.sender != curr_handler {
        return Err(ContractError::Unauthorized {});
    }
    if nonce <= proposal_nonce || proposal_nonce + 1048 < nonce {
        return Err(ContractError::InvalidNonce);
    }

    // Save a new "handler"
    let new_handler = deps.api.addr_validate(&handler)?;
    anchor.handler = new_handler;
    anchor.proposal_nonce = nonce;

    ANCHOR.save(deps.storage, &anchor)?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "set_handler"),
        attr("handler", handler),
        attr("nonce", nonce.to_string()),
    ]))
}

/// Add an edge to underlying linkable tree
fn add_edge(
    deps: DepsMut,
    src_chain_id: u64,
    root: [u8; 32],
    latest_leaf_index: u32,
    target: [u8; 32],
) -> Result<Response, ContractError> {
    let linkable_tree = ANCHOR.load(deps.storage)?.linkable_tree;

    // Ensure anchor isn't at maximum edges
    let curr_length = linkable_tree.get_latest_neighbor_edges(deps.storage).len();
    if curr_length > linkable_tree.max_edges as usize {
        return Err(ContractError::TooManyEdges {});
    }

    // Add new edge to the end of the edge list for the given tree
    let edge: Edge = Edge {
        src_chain_id,
        root,
        latest_leaf_index,
        target,
    };
    save_edge(deps.storage, src_chain_id, edge)?;

    // Update associated states
    let curr_neighbor_root_idx = read_curr_neighbor_root_index(deps.storage, src_chain_id)?;
    save_curr_neighbor_root_index(
        deps.storage,
        src_chain_id,
        (curr_neighbor_root_idx + 1) % HISTORY_LENGTH,
    )?;

    save_neighbor_roots(deps.storage, (src_chain_id, curr_neighbor_root_idx), root)?;

    Ok(
        Response::new().add_event(Event::new("anchor-edge_add").add_attributes(vec![
            attr("action", "add_edge"),
            attr("src_chain_id", src_chain_id.to_string()),
            attr("leaf_index", latest_leaf_index.to_string()),
            attr("root", format!("{:?}", root)),
        ])),
    )
}

/// Update an edge for underlying linkable tree
fn update_edge(
    deps: DepsMut,
    src_chain_id: u64,
    root: [u8; 32],
    latest_leaf_index: u32,
    target: [u8; 32],
) -> Result<Response, ContractError> {
    // Update an existing edge with new one
    let edge: Edge = Edge {
        src_chain_id,
        root,
        latest_leaf_index,
        target,
    };
    save_edge(deps.storage, src_chain_id, edge)?;

    // Update associated states
    let neighbor_root_idx =
        (read_curr_neighbor_root_index(deps.storage, src_chain_id)? + 1) % HISTORY_LENGTH;
    save_curr_neighbor_root_index(deps.storage, src_chain_id, neighbor_root_idx)?;

    save_neighbor_roots(deps.storage, (src_chain_id, neighbor_root_idx), root)?;

    Ok(
        Response::new().add_event(Event::new("anchor-edge_update").add_attributes(vec![
            attr("action", "update_edge"),
            attr("src_chain_id", src_chain_id.to_string()),
            attr("leaf_index", latest_leaf_index.to_string()),
            attr("root", format!("{:?}", root)),
        ])),
    )
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&get_config(deps)?),
        QueryMsg::EdgeInfo { id } => to_binary(&get_edge_info(deps, id)?),
        QueryMsg::NeighborRootInfo { chain_id, id } => {
            to_binary(&get_neighbor_root_info(deps, chain_id, id)?)
        }
        QueryMsg::MerkleTreeInfo {} => to_binary(&get_merkle_tree_info(deps)?),
        QueryMsg::MerkleRootInfo { id } => to_binary(&get_merkle_root(deps, id)?),
    }
}

pub fn get_config(deps: Deps) -> StdResult<ConfigResponse> {
    let anchor = ANCHOR.load(deps.storage)?;
    Ok(ConfigResponse {
        handler: anchor.handler.to_string(),
        proposal_nonce: anchor.proposal_nonce,
        tokenwrapper_addr: anchor.tokenwrapper_addr.to_string(),
        deposit_size: anchor.deposit_size.to_string(),
    })
}

pub fn get_edge_info(deps: Deps, id: u64) -> StdResult<EdgeInfoResponse> {
    let edge = read_edge(deps.storage, id)?;
    Ok(EdgeInfoResponse {
        src_chain_id: edge.src_chain_id,
        root: edge.root,
        latest_leaf_index: edge.latest_leaf_index,
        target: edge.target,
    })
}

pub fn get_neighbor_root_info(
    deps: Deps,
    chain_id: u64,
    id: u32,
) -> StdResult<NeighborRootInfoResponse> {
    let neighbor_root = read_neighbor_roots(deps.storage, (chain_id, id))?;
    Ok(NeighborRootInfoResponse { neighbor_root })
}

pub fn get_merkle_tree_info(deps: Deps) -> StdResult<MerkleTreeInfoResponse> {
    let anchor = ANCHOR.load(deps.storage)?;
    Ok(MerkleTreeInfoResponse {
        levels: anchor.merkle_tree.levels,
        curr_root_index: anchor.merkle_tree.current_root_index,
        next_index: anchor.merkle_tree.next_index,
    })
}

pub fn get_merkle_root(deps: Deps, id: u32) -> StdResult<MerkleRootInfoResponse> {
    let root = read_root(deps.storage, id)?;
    Ok(MerkleRootInfoResponse { root })
}

// Check if the "nullifier" is already used or not.
pub fn is_known_nullifier(store: &dyn Storage, nullifier: [u8; 32]) -> bool {
    NULLIFIERS.has(store, nullifier.to_vec())
}

// Using "anchor_verifier", verifies if the "proof" really came from "public_input".
pub fn verify(
    verifier: AnchorVerifier,
    public_input: Vec<u8>,
    proof_bytes: Vec<u8>,
) -> Result<bool, ContractError> {
    verifier
        .verify(public_input, proof_bytes)
        .map_err(|_| ContractError::VerifyError)
}

pub fn validate_and_store_commitment(
    deps: DepsMut,
    commitment: [u8; 32],
) -> Result<u32, ContractError> {
    let anchor = ANCHOR.load(deps.storage)?;
    let mut merkle_tree = anchor.merkle_tree;
    let poseidon = HASHER.load(deps.storage)?;
    let res = merkle_tree
        .insert(poseidon, commitment, deps.storage)
        .map_err(|_| ContractError::MerkleTreeIsFull)?;

    ANCHOR.save(
        deps.storage,
        &Anchor {
            deposit_size: anchor.deposit_size,
            linkable_tree: anchor.linkable_tree,
            tokenwrapper_addr: anchor.tokenwrapper_addr,
            handler: anchor.handler,
            proposal_nonce: anchor.proposal_nonce,
            merkle_tree,
        },
    )?;

    Ok(res)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    Ok(Response::default())
}
