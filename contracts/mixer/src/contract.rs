#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, from_binary, to_binary, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, Event,
    MessageInfo, Response, StdError, StdResult, Storage, WasmMsg,
};
use cw2::set_contract_version;

use protocol_cosmwasm::error::ContractError;
use protocol_cosmwasm::keccak::Keccak256;
use protocol_cosmwasm::mixer::{
    ConfigResponse, Cw20HookMsg, DepositMsg, ExecuteMsg, InstantiateMsg, MerkleRootResponse,
    MerkleTreeInfoResponse, QueryMsg, WithdrawMsg,
};
use protocol_cosmwasm::mixer_verifier::MixerVerifier;
use protocol_cosmwasm::poseidon::Poseidon;
use protocol_cosmwasm::utils::truncate_and_pad;
use protocol_cosmwasm::zeroes::zeroes;

use codec::Encode;
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};

use crate::state::{
    read_root, save_root, save_subtree, MerkleTree, Mixer, MIXER, MIXERVERIFIER, POSEIDON,
    USED_NULLIFIERS,
};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cosmwasm-mixer";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

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

    // Initialize the "Mixer"
    let merkle_tree: MerkleTree = MerkleTree {
        levels: msg.merkletree_levels,
        current_root_index: 0,
        next_index: 0,
    };
    let native_token_denom = msg.native_token_denom;
    let cw20_address = match msg.cw20_address {
        Some(addr) => Some(deps.api.addr_validate(addr.as_str())?),
        None => None,
    };
    if native_token_denom.is_some() && cw20_address.is_some() {
        return Err(ContractError::Std(StdError::GenericErr {
            msg: "Both the native_token_denom and cw20_address cannot be set at the same time"
                .to_string(),
        }));
    }
    if native_token_denom.is_none() && cw20_address.is_none() {
        return Err(ContractError::Std(StdError::GenericErr {
            msg: "Both the native_token_denom and cw20_address cannot be empty at the same time"
                .to_string(),
        }));
    }
    let deposit_size = msg.deposit_size;

    let mixer: Mixer = Mixer {
        cw20_address,
        native_token_denom,
        deposit_size,
        merkle_tree,
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
        .add_attribute("action", "instantiate")
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
        // Deposit the "native" tokens with commitment
        ExecuteMsg::Deposit(msg) => deposit_native(deps, info, msg),
        // Withdraw either "native" tokens or cw20 tokens.
        ExecuteMsg::Withdraw(msg) => withdraw(deps, info, msg),
        // Deposit the cw20 tokens with commitment
        ExecuteMsg::Receive(msg) => receive_cw20(deps, info, msg),
    }
}

pub fn deposit_native(
    deps: DepsMut,
    info: MessageInfo,
    msg: DepositMsg,
) -> Result<Response, ContractError> {
    let mixer = MIXER.load(deps.storage)?;

    // Validations
    if mixer.native_token_denom.is_none() {
        return Err(ContractError::Std(StdError::GenericErr {
            msg: "This mixer is for native tokens".to_string(),
        }));
    }
    let native_token_denom = mixer.native_token_denom.unwrap();
    let sent_tokens: Vec<Coin> = info
        .funds
        .into_iter()
        .filter(|x| x.denom == native_token_denom)
        .collect();
    if sent_tokens.is_empty() || sent_tokens[0].amount < mixer.deposit_size {
        return Err(ContractError::InsufficientFunds {});
    }

    // Handle the "deposit"
    if let Some(commitment) = msg.commitment {
        let mut merkle_tree = mixer.merkle_tree;
        let poseidon = POSEIDON.load(deps.storage)?;
        let inserted_index = merkle_tree.insert(poseidon, commitment, deps.storage)?;
        MIXER.save(
            deps.storage,
            &Mixer {
                native_token_denom: Some(native_token_denom),
                cw20_address: mixer.cw20_address,
                deposit_size: mixer.deposit_size,
                merkle_tree,
            },
        )?;
        Ok(
            Response::new().add_event(Event::new("mixer-deposit").add_attributes(vec![
                attr("action", "deposit_native"),
                attr("inserted_index", inserted_index.to_string()),
                attr("commitment", format!("{:?}", commitment)),
            ])),
        )
    } else {
        Err(ContractError::Std(StdError::NotFound {
            kind: "Commitment".to_string(),
        }))
    }
}

pub fn receive_cw20(
    deps: DepsMut,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let mixer: Mixer = MIXER.load(deps.storage)?;

    // Validations
    if mixer.cw20_address.is_none() {
        return Err(ContractError::Std(StdError::GenericErr {
            msg: "This mixer is for cw20 token".to_string(),
        }));
    }
    let cw20_address = mixer.cw20_address.unwrap();
    if cw20_address != deps.api.addr_validate(info.sender.as_str())? {
        return Err(ContractError::Unauthorized {});
    }

    let sent_cw20_token_amt = cw20_msg.amount;
    if sent_cw20_token_amt < mixer.deposit_size {
        return Err(ContractError::InsufficientFunds {});
    }

    match from_binary(&cw20_msg.msg) {
        Ok(Cw20HookMsg::DepositCw20 { commitment }) => {
            // Handle the "deposit"
            if let Some(commitment) = commitment {
                let mut merkle_tree = mixer.merkle_tree;
                let poseidon = POSEIDON.load(deps.storage)?;
                let inserted_index = merkle_tree
                    .insert(poseidon, commitment, deps.storage)
                    .map_err(|_| ContractError::MerkleTreeIsFull)?;

                MIXER.save(
                    deps.storage,
                    &Mixer {
                        native_token_denom: mixer.native_token_denom,
                        cw20_address: Some(cw20_address),
                        deposit_size: mixer.deposit_size,
                        merkle_tree,
                    },
                )?;

                Ok(
                    Response::new().add_event(Event::new("mixer-deposit").add_attributes(vec![
                        attr("action", "deposit_cw20"),
                        attr("inserted_index", inserted_index.to_string()),
                        attr("commitment", format!("{:?}", commitment)),
                    ])),
                )
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

pub fn withdraw(
    deps: DepsMut,
    info: MessageInfo,
    msg: WithdrawMsg,
) -> Result<Response, ContractError> {
    let recipient = deps.api.addr_validate(msg.recipient.as_str())?.to_string();
    let relayer = deps.api.addr_validate(msg.relayer.as_str())?.to_string();
    let fee = msg.fee;
    let refund = msg.refund;

    let mixer = MIXER.load(deps.storage)?;

    // Validations
    let sent_funds = info.funds;
    if !refund.is_zero() && (sent_funds.len() != 1 || sent_funds[0].amount != refund) {
        return Err(ContractError::Std(StdError::GenericErr {
            msg: "Sent insufficent refund".to_string(),
        }));
    }

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
    let recipient_bytes = truncate_and_pad(recipient.as_bytes());
    let relayer_bytes = truncate_and_pad(relayer.as_bytes());

    let mut arbitrary_data_bytes = Vec::new();
    arbitrary_data_bytes.extend_from_slice(&recipient_bytes);
    arbitrary_data_bytes.extend_from_slice(&relayer_bytes);
    arbitrary_data_bytes.extend_from_slice(&fee.u128().encode());
    arbitrary_data_bytes.extend_from_slice(&refund.u128().encode());
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
    let amt_to_recipient = match mixer.deposit_size.checked_sub(fee) {
        Ok(v) => v,
        Err(e) => {
            return Err(ContractError::Std(StdError::GenericErr {
                msg: e.to_string(),
            }))
        }
    };

    // If the "cw20_address" is set, then send the Cw20 tokens.
    // Otherwise, send the native tokens.
    if let Some(cw20_address) = msg.cw20_address {
        // Validate the "cw20_address".
        if mixer.cw20_address.unwrap() != deps.api.addr_validate(cw20_address.as_str())? {
            return Err(ContractError::Std(StdError::GenericErr {
                msg: "Invalid cw20 address".to_string(),
            }));
        }
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
    } else {
        let native_token_denom = mixer.native_token_denom.unwrap();
        if !amt_to_recipient.is_zero() {
            msgs.push(CosmosMsg::Bank(BankMsg::Send {
                to_address: recipient.clone(),
                amount: vec![Coin {
                    denom: native_token_denom.clone(),
                    amount: amt_to_recipient,
                }],
            }));
        }
        if !fee.is_zero() {
            msgs.push(CosmosMsg::Bank(BankMsg::Send {
                to_address: relayer,
                amount: vec![Coin {
                    denom: native_token_denom,
                    amount: fee,
                }],
            }));
        }
    }

    if !refund.is_zero() {
        msgs.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: recipient.clone(),
            amount: sent_funds,
        }));
    }

    Ok(Response::new()
        .add_messages(msgs)
        .add_event(Event::new("mixer-withdraw").add_attributes(vec![
            attr("action", "withdraw"),
            attr("recipient", recipient),
            attr("root", format!("{:?}", msg.root)),
            attr("nullifier_hash", format!("{:?}", msg.nullifier_hash)),
        ])))
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

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&get_config(deps)?),
        QueryMsg::MerkleTreeInfo {} => to_binary(&get_merkle_tree_info(deps)?),
        QueryMsg::MerkleRoot { id } => to_binary(&get_merkle_root(deps, id)?),
    }
}

fn get_config(deps: Deps) -> StdResult<ConfigResponse> {
    let mixer = MIXER.load(deps.storage)?;
    let native_token_denom = match mixer.native_token_denom {
        Some(v) => v,
        None => "".to_string(),
    };
    let cw20_address = match mixer.cw20_address {
        Some(v) => v.to_string(),
        None => "".to_string(),
    };
    let deposit_size = mixer.deposit_size.to_string();
    Ok(ConfigResponse {
        native_token_denom,
        cw20_address,
        deposit_size,
    })
}

fn get_merkle_tree_info(deps: Deps) -> StdResult<MerkleTreeInfoResponse> {
    let mixer = MIXER.load(deps.storage)?;
    Ok(MerkleTreeInfoResponse {
        levels: mixer.merkle_tree.levels,
        current_root_index: mixer.merkle_tree.current_root_index,
        next_index: mixer.merkle_tree.next_index,
    })
}

fn get_merkle_root(deps: Deps, id: u32) -> StdResult<MerkleRootResponse> {
    let root = read_root(deps.storage, id)?;
    Ok(MerkleRootResponse { root })
}
