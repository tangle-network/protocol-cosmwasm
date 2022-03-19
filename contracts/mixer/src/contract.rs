#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, from_binary, to_binary, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env,
    MessageInfo, Response, StdError, StdResult, Storage, Uint128, Uint256, WasmMsg,
};
use cw2::set_contract_version;
use std::convert::TryFrom;

use protocol_cosmwasm::error::ContractError;
use protocol_cosmwasm::keccak::Keccak256;
use protocol_cosmwasm::mixer::{
    Cw20HookMsg, DepositMsg, ExecuteMsg, InfoResponse, InstantiateMsg, QueryMsg, WithdrawMsg,
};
use protocol_cosmwasm::mixer_verifier::MixerVerifier;
use protocol_cosmwasm::poseidon::Poseidon;
use protocol_cosmwasm::zeroes::zeroes;

use codec::Encode;
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};

use crate::state::{
    save_root, save_subtree, MerkleTree, Mixer, MIXER, MIXERVERIFIER, POSEIDON, USED_NULLIFIERS,
};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cosmwasm-mixer";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// User instantiates the "mixer" contract.
/// IMPORTANT:
///     Every individual mixer is for either native(UST) token or CW20 token.
///     For example, when instantiating:
///         If the "cw20_address" field is empty, then the mixer is for native(UST) token.
///         If the "cw20_address" field is set,   then the mixer is for CW20 token.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    // Validation 1. Check if the funds are sent with this message
    if !info.funds.is_empty() {
        return Err(ContractError::UnnecessaryFunds {});
    }

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Initialize the merkle tree
    let merkle_tree: MerkleTree = MerkleTree {
        levels: msg.merkletree_levels,
        current_root_index: 0,
        next_index: 0,
    };

    // Check the validity of "cw20_address" if exists.
    let cw20_address = match msg.cw20_address {
        Some(addr) => Some(deps.api.addr_canonicalize(addr.as_str())?),
        None => None,
    };

    // Initialize the Mixer
    let mixer: Mixer = Mixer {
        initialized: true,
        deposit_size: Uint256::from(msg.deposit_size.u128()),
        merkle_tree,
        cw20_address,
    };
    MIXER.save(deps.storage, &mixer)?;

    // Initialize the poseidon hasher
    POSEIDON.save(deps.storage, &Poseidon::new())?;

    // Initialize the Mixer_Verifier
    MIXERVERIFIER.save(deps.storage, &MixerVerifier::new())?;

    for i in 0..msg.merkletree_levels {
        save_subtree(deps.storage, i as u32, &zeroes(i))?;
    }

    save_root(deps.storage, 0_u32, &zeroes(msg.merkletree_levels))?;

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
        ExecuteMsg::Receive(msg) => receive_cw20(deps, info, msg),
    }
}

/// User deposits the fund(UST) with its commitment.
/// It checks the validity of the fund(UST) sent.
/// It also checks the merkle tree availiability.
/// It saves the commitment in "merkle tree".
pub fn deposit(
    deps: DepsMut,
    info: MessageInfo,
    msg: DepositMsg,
) -> Result<Response, ContractError> {
    let mixer = MIXER.load(deps.storage)?;

    // Validation 1. Check if the enough UST are sent.
    let sent_uusd: Vec<Coin> = info
        .funds
        .into_iter()
        .filter(|x| x.denom == "uusd")
        .collect();
    if sent_uusd.is_empty() || Uint256::from(sent_uusd[0].amount) < mixer.deposit_size {
        return Err(ContractError::InsufficientFunds {});
    }

    // Validation 2. Check if the mixer is initialized
    if !mixer.initialized {
        return Err(ContractError::NotInitialized {});
    }

    // Validation 3. Check if the mixer is for native(UST) token.
    if mixer.cw20_address.is_some() {
        return Err(ContractError::Std(StdError::generic_err(
            "This mixer is for CW20 token",
        )));
    }

    if let Some(commitment) = msg.commitment {
        let mut merkle_tree = mixer.merkle_tree;
        let poseidon = POSEIDON.load(deps.storage)?;
        let res = merkle_tree.insert(poseidon, commitment, deps.storage)?;
        MIXER.save(
            deps.storage,
            &Mixer {
                initialized: mixer.initialized,
                deposit_size: mixer.deposit_size,
                cw20_address: mixer.cw20_address,
                merkle_tree,
            },
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
    let mixer: Mixer = MIXER.load(deps.storage)?;
    let cw20_address = mixer.cw20_address.clone();
    if cw20_address.is_none() {
        return Err(ContractError::Std(StdError::generic_err(
            "This mixer is for native(UST) token",
        )));
    }

    if cw20_address.unwrap() != deps.api.addr_canonicalize(info.sender.as_str())? {
        return Err(ContractError::Unauthorized {});
    }

    let tokens_sent = cw20_msg.amount;
    match from_binary(&cw20_msg.msg) {
        Ok(Cw20HookMsg::DepositCw20 { commitment }) => {
            if Uint256::from(tokens_sent) < mixer.deposit_size {
                return Err(ContractError::InsufficientFunds {});
            }
            // Checks the validity of
            if let Some(commitment) = commitment {
                let mut merkle_tree = mixer.merkle_tree;
                let poseidon = POSEIDON.load(deps.storage)?;
                let res = merkle_tree
                    .insert(poseidon, commitment, deps.storage)
                    .map_err(|_| ContractError::MerkleTreeIsFull)?;

                MIXER.save(
                    deps.storage,
                    &Mixer {
                        initialized: mixer.initialized,
                        deposit_size: mixer.deposit_size,
                        cw20_address: mixer.cw20_address,
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

/// User withdraws the native(UST) token or CW20 token
/// to "recipient" address by providing the "proof" for
/// the "commitment".
/// It verifies the "withdraw" by verifying the "proof"
/// with "commitment" saved in prior.
/// If success on verify, then it performs "withdraw" action
/// which sends the native(UST) token or CW20 token
/// to "recipient" & "relayer" address.
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
    let mixer = MIXER.load(deps.storage)?;
    let merkle_tree = mixer.merkle_tree;

    if !merkle_tree.is_known_root(msg.root, deps.storage) {
        return Err(ContractError::Std(StdError::GenericErr {
            msg: "Root is not known".to_string(),
        }));
    }

    if is_known_nullifier(deps.storage, msg.nullifier_hash) {
        return Err(ContractError::Std(StdError::GenericErr {
            msg: "Nullifier is known".to_string(),
        }));
    }

    // Format the public input bytes
    let recipient_bytes =
        truncate_and_pad(&hex::decode(&msg.recipient).map_err(|_| ContractError::DecodeError)?);
    let relayer_bytes =
        truncate_and_pad(&hex::decode(&msg.relayer).map_err(|_| ContractError::DecodeError)?);
    let fee_u128 = match Uint128::try_from(msg.fee) {
        Ok(v) => v,
        Err(_) => {
            return Err(ContractError::Std(StdError::GenericErr {
                msg: "Cannot convert fee".to_string(),
            }))
        }
    };
    let refund_u128 = match Uint128::try_from(msg.refund) {
        Ok(v) => v,
        Err(_) => {
            return Err(ContractError::Std(StdError::GenericErr {
                msg: "Cannot convert refund".to_string(),
            }))
        }
    };

    let mut arbitrary_data_bytes = Vec::new();
    arbitrary_data_bytes.extend_from_slice(&recipient_bytes);
    arbitrary_data_bytes.extend_from_slice(&relayer_bytes);
    arbitrary_data_bytes.extend_from_slice(&fee_u128.u128().encode());
    arbitrary_data_bytes.extend_from_slice(&refund_u128.u128().encode());
    let arbitrary_input =
        Keccak256::hash(&arbitrary_data_bytes).map_err(|_| ContractError::HashError)?;

    // Join the public input bytes
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&msg.nullifier_hash);
    bytes.extend_from_slice(&msg.root);
    bytes.extend_from_slice(&arbitrary_input);
    // Verify the proof
    let verifier = MIXERVERIFIER.load(deps.storage)?;
    let result = verify(verifier, bytes, msg.proof_bytes)?;

    if !result {
        return Err(ContractError::Std(StdError::GenericErr {
            msg: "Invalid withdraw proof".to_string(),
        }));
    }

    // Set used nullifier to true after successful verification
    USED_NULLIFIERS.save(deps.storage, msg.nullifier_hash.to_vec(), &true)?;

    // Send the funds
    let mut msgs: Vec<CosmosMsg> = vec![];

    // Send the funds to "recipient"
    let amt_to_recipient = match Uint128::try_from(mixer.deposit_size - msg.fee) {
        Ok(v) => v,
        Err(_) => {
            return Err(ContractError::Std(StdError::GenericErr {
                msg: "Cannot compute amount".to_string(),
            }))
        }
    };

    // Send the funds to "relayer"
    let amt_to_relayer = match Uint128::try_from(msg.fee) {
        Ok(v) => v,
        Err(_) => {
            return Err(ContractError::Std(StdError::GenericErr {
                msg: "Cannot compute amount".to_string(),
            }))
        }
    };

    // If "refund" field is non-zero, send the funds to "recipient"
    let mut amt_refund: Uint128 = Uint128::zero();
    if msg.refund > Uint256::zero() {
        amt_refund = match Uint128::try_from(msg.refund) {
            Ok(v) => v,
            Err(_) => {
                return Err(ContractError::Std(StdError::GenericErr {
                    msg: "Cannot compute amount".to_string(),
                }))
            }
        };
    }

    // If the "cw20_address" is set, then send the Cw20 tokens.
    // Otherwise, send the native tokens.
    if let Some(cw20_address) = msg.cw20_address {
        // Validate the "cw20_address".
        if mixer.cw20_address.unwrap() != deps.api.addr_canonicalize(cw20_address.as_str())? {
            return Err(ContractError::Std(StdError::generic_err(
                "Invalid cw20 address",
            )));
        }
        let cw20_token_contract = cw20_address;

        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cw20_token_contract.clone(),
            funds: [].to_vec(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: msg.recipient.clone(),
                amount: amt_to_recipient,
            })?,
        }));

        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cw20_token_contract.clone(),
            funds: [].to_vec(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: msg.relayer.clone(),
                amount: amt_to_relayer,
            })?,
        }));

        if msg.refund > Uint256::zero() {
            msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: cw20_token_contract,
                funds: [].to_vec(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: msg.recipient.clone(),
                    amount: amt_refund,
                })?,
            }));
        }
    } else {
        msgs.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: msg.recipient.clone(),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: amt_to_recipient,
            }],
        }));

        msgs.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: msg.relayer,
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: amt_to_relayer,
            }],
        }));

        if msg.refund > Uint256::zero() {
            msgs.push(CosmosMsg::Bank(BankMsg::Send {
                to_address: msg.recipient,
                amount: vec![Coin {
                    denom: "uusd".to_string(),
                    amount: amt_refund,
                }],
            }));
        }
    }

    Ok(Response::new()
        .add_attributes(vec![attr("method", "withdraw")])
        .add_messages(msgs))
}

fn is_known_nullifier(store: &dyn Storage, nullifier: [u8; 32]) -> bool {
    USED_NULLIFIERS.has(store, nullifier.to_vec())
}

fn verify(
    verifier: MixerVerifier,
    public_input: Vec<u8>,
    proof_bytes: Vec<u8>,
) -> Result<bool, ContractError> {
    verifier
        .verify(public_input, proof_bytes)
        .map_err(|_| ContractError::VerifyError)
}

pub fn truncate_and_pad(t: &[u8]) -> Vec<u8> {
    let mut truncated_bytes = t[..20].to_vec();
    truncated_bytes.extend_from_slice(&[0u8; 12]);
    truncated_bytes
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetCw20Address {} => to_binary(&get_cw20_address(deps)?),
    }
}

fn get_cw20_address(deps: Deps) -> StdResult<InfoResponse> {
    let mixer = MIXER.load(deps.storage)?;

    let cw20_address = match mixer.cw20_address {
        Some(cw20_address) => deps.api.addr_humanize(&cw20_address)?.to_string(),
        None => "".to_string(),
    };

    Ok(InfoResponse { cw20_address })
}
