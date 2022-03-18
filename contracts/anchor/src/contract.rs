#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, from_binary, to_binary, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response,
    StdError, StdResult, Storage, Uint128, Uint256, WasmMsg,
};
use cw2::set_contract_version;

use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};
use protocol_cosmwasm::anchor::{
    Cw20HookMsg, ExecuteMsg, InfoResponse, InstantiateMsg, QueryMsg, WithdrawMsg,
};
use protocol_cosmwasm::anchor_verifier::AnchorVerifier;
use protocol_cosmwasm::error::ContractError;
use protocol_cosmwasm::poseidon::Poseidon;
use protocol_cosmwasm::zeroes::zeroes;

use crate::state::{
    save_root, save_subtree, Anchor, LinkableMerkleTree, MerkleTree, ANCHOR, ANCHORVERIFIER,
    NULLIFIERS, POSEIDON,
};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cosmwasm-anchor";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// ChainType info
pub const COSMOS_CHAIN_TYPE: [u8; 2] = [4, 0]; // 0x0400

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

    // Get the "cw20_address"
    let cw20_address = deps.api.addr_canonicalize(&msg.cw20_address)?;

    // Initialize the Anchor
    let anchor = Anchor {
        chain_id: msg.chain_id,
        deposit_size: Uint256::from(msg.deposit_size.u128()),
        linkable_tree: linkable_merkle_tree,
        merkle_tree,
        cw20_address,
    };
    ANCHOR.save(deps.storage, &anchor)?;

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
        ExecuteMsg::Withdraw(msg) => withdraw(deps, info, msg),
        ExecuteMsg::Receive(msg) => receive_cw20(deps, info, msg),
    }
}

/// User deposits the Cw20 tokens with its commitments.
/// The deposit starts from executing the hook message
/// coming from the Cw20 token contract.
/// It checks the validity of the Cw20 tokens sent.
/// It also checks the merkle tree availiability.
/// It saves the commitment in "merkle tree".
pub fn receive_cw20(
    deps: DepsMut,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    // Only Cw20 token contract can execute this message.
    let anchor: Anchor = ANCHOR.load(deps.storage)?;
    if anchor.cw20_address != deps.api.addr_canonicalize(info.sender.as_str())? {
        return Err(ContractError::Unauthorized {});
    }

    let tokens_sent = cw20_msg.amount;
    match from_binary(&cw20_msg.msg) {
        Ok(Cw20HookMsg::DepositCw20 { commitment }) => {
            if Uint256::from(tokens_sent) < anchor.deposit_size {
                return Err(ContractError::InsufficientFunds {});
            }
            // Checks the validity of
            if let Some(commitment) = commitment {
                let mut merkle_tree = anchor.merkle_tree;
                let poseidon = POSEIDON.load(deps.storage)?;
                let res = merkle_tree
                    .insert(poseidon, commitment, deps.storage)
                    .map_err(|_| ContractError::MerkleTreeIsFull)?;

                ANCHOR.save(
                    deps.storage,
                    &Anchor {
                        chain_id: anchor.chain_id,
                        deposit_size: anchor.deposit_size,
                        linkable_tree: anchor.linkable_tree,
                        cw20_address: anchor.cw20_address,
                        merkle_tree,
                    },
                )?;

                Ok(Response::new().add_attributes(vec![
                    attr("method", "deposit_cw20"),
                    attr("result", res.to_string()),
                ]))
            } else {
                Err(ContractError::Std(StdError::NotFound {
                    kind: "Commitment".to_string(),
                }))
            }
        }
        Err(_) => Err(ContractError::Std(StdError::generic_err(
            "invalid cw20 hook msg",
        ))),
    }
}

/// User withdraws the CW20 tokens to "recipient" address
/// by providing the "proof" for the "commitment".
/// It verifies the "withdraw" by verifying the "proof"
/// with "commitment" saved in prior.
/// If success on verify, then it performs "withdraw" action
/// which sends the CW20 tokens to "recipient" & "relayer" address.
pub fn withdraw(
    deps: DepsMut,
    info: MessageInfo,
    msg: WithdrawMsg,
) -> Result<Response, ContractError> {
    // Validation 1. Check if the funds are sent.
    if !info.funds.is_empty() {
        return Err(ContractError::UnnecessaryFunds {});
    }

    // Validation 2. Check if the root is known to merkle tree
    let anchor = ANCHOR.load(deps.storage)?;

    let merkle_tree = anchor.merkle_tree;
    if !merkle_tree.is_known_root(msg.roots[0], deps.storage) {
        return Err(ContractError::Std(StdError::GenericErr {
            msg: "Root is not known".to_string(),
        }));
    }

    // Validation 3. Check if the roots are valid in linkable tree.
    let linkable_tree = anchor.linkable_tree;
    if !linkable_tree.is_valid_neighbor_roots(&msg.roots[1..], deps.storage) {
        return Err(ContractError::Std(StdError::GenericErr {
            msg: "Neighbor roots are not valid".to_string(),
        }));
    }

    // Checks if the nullifier already used.
    if is_known_nullifier(deps.storage, msg.nullifier_hash) {
        return Err(ContractError::Std(StdError::GenericErr {
            msg: "Nullifier is known".to_string(),
        }));
    }

    let element_encoder = |v: &[u8]| {
        let mut output = [0u8; 32];
        output.iter_mut().zip(v).for_each(|(b1, b2)| *b1 = *b2);
        output
    };

    // Format the public input bytes
    let chain_id_type_bytes =
        element_encoder(&compute_chain_id_type(anchor.chain_id, &COSMOS_CHAIN_TYPE).to_le_bytes());
    let recipient_bytes =
        truncate_and_pad(&hex::decode(&msg.recipient).map_err(|_| ContractError::DecodeError)?);
    let relayer_bytes =
        truncate_and_pad(&hex::decode(&msg.relayer).map_err(|_| ContractError::DecodeError)?);
    let fee_bytes = element_encoder(&msg.fee.to_le_bytes());
    let refund_bytes = element_encoder(&msg.refund.to_le_bytes());

    // Join the public input bytes
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&chain_id_type_bytes);
    bytes.extend_from_slice(&msg.nullifier_hash);
    for root in msg.roots {
        bytes.extend_from_slice(&root);
    }
    bytes.extend_from_slice(&recipient_bytes);
    bytes.extend_from_slice(&relayer_bytes);
    bytes.extend_from_slice(&fee_bytes);
    bytes.extend_from_slice(&refund_bytes);
    bytes.extend_from_slice(&msg.commitment);

    // Verify the proof
    let verifier = ANCHORVERIFIER.load(deps.storage)?;
    let result = verify(verifier, bytes, msg.proof_bytes)?;

    if !result {
        return Err(ContractError::Std(StdError::GenericErr {
            msg: "Invalid withdraw proof".to_string(),
        }));
    }

    // Set used nullifier to true after successful verification
    NULLIFIERS.save(deps.storage, msg.nullifier_hash.to_vec(), &true)?;

    // Validate the "cw20_address".
    let cw20_address = msg.cw20_address;
    if anchor.cw20_address != deps.api.addr_canonicalize(cw20_address.as_str())? {
        return Err(ContractError::Std(StdError::generic_err(
            "Invalid cw20 address",
        )));
    }

    // Send the funds
    let mut msgs: Vec<CosmosMsg> = vec![];

    // Send the funds to "recipient"
    let amt_to_recipient = match Uint128::try_from(anchor.deposit_size - msg.fee) {
        Ok(v) => v,
        Err(_) => {
            return Err(ContractError::Std(StdError::GenericErr {
                msg: "Cannot compute amount".to_string(),
            }))
        }
    };
    msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: cw20_address.clone(),
        funds: [].to_vec(),
        msg: to_binary(&Cw20ExecuteMsg::Transfer {
            recipient: msg.recipient.clone(),
            amount: amt_to_recipient,
        })?,
    }));

    // Send the funds to "relayer"
    let amt_to_relayer = match Uint128::try_from(msg.fee) {
        Ok(v) => v,
        Err(_) => {
            return Err(ContractError::Std(StdError::GenericErr {
                msg: "Cannot compute amount".to_string(),
            }))
        }
    };
    msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: cw20_address.clone(),
        funds: [].to_vec(),
        msg: to_binary(&Cw20ExecuteMsg::Transfer {
            recipient: msg.relayer.clone(),
            amount: amt_to_relayer,
        })?,
    }));

    // If "refund" field is non-zero, send the funds to "recipient"
    if msg.refund > Uint256::zero() {
        let amt_refund = match Uint128::try_from(msg.refund) {
            Ok(v) => v,
            Err(_) => {
                return Err(ContractError::Std(StdError::GenericErr {
                    msg: "Cannot compute amount".to_string(),
                }))
            }
        };
        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cw20_address,
            funds: [].to_vec(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: msg.recipient.clone(),
                amount: amt_refund,
            })?,
        }));
    }

    Ok(Response::new()
        .add_attributes(vec![attr("method", "withdraw")])
        .add_messages(msgs))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetCw20Address {} => to_binary(&get_cw20_address(deps)?),
    }
}

pub fn get_cw20_address(deps: Deps) -> StdResult<InfoResponse> {
    let anchor = ANCHOR.load(deps.storage)?;
    Ok(InfoResponse {
        cw20_address: deps.api.addr_humanize(&anchor.cw20_address)?.to_string(),
    })
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

// Truncate and pad 256 bit slice
pub fn truncate_and_pad(t: &[u8]) -> Vec<u8> {
    let mut truncated_bytes = t[..20].to_vec();
    truncated_bytes.extend_from_slice(&[0u8; 12]);
    truncated_bytes
}

// Computes the combination bytes of "chain_type" and "chain_id".
// Combination rule: 8 bytes array(00 * 2 bytes + [chain_type] 2 bytes + [chain_id] 4 bytes)
// Example:
//  chain_type - 0x0401, chain_id - 0x00000001 (big endian)
//  Result - [00, 00, 04, 01, 00, 00, 00, 01]
pub fn compute_chain_id_type(chain_id: u64, chain_type: &[u8]) -> u64 {
    let chain_id_value: u32 = chain_id.try_into().unwrap_or_default();
    let mut buf = [0u8; 8];
    #[allow(clippy::needless_borrow)]
    buf[2..4].copy_from_slice(&chain_type);
    buf[4..8].copy_from_slice(&chain_id_value.to_be_bytes());
    u64::from_be_bytes(buf)
}
