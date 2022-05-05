use cosmwasm_std::Uint128;
/**
   @title linkable_anchor(ILinkableAnchor) Interface
   @notice The interface supports updating edges for a graph-like functionality.
   It also supports setting handlers and verifiers for handling updates
   to the edge data of a LinkableAnchor as well as the verifier used in
   verifying proofs of knowledge of leaves in one-of-many merkle trees.

   The ILinkableAnchor interface can also be used with the VAnchor system
   to control the minimal and maximum withdrawal and deposit limits respectively.
*/

/**
 * NOTE: The detail of these interfaces is prone to future change,
 *       since the interfaces are just for creating the call to various
 *       kinds of "(v)anchor" contracts.
 */
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /**
       @notice Sets the handler for updating edges and other contract state
       @param handler The new handler address
       @param nonce The nonce for tracking update counts
    */
    SetHandler { handler: String, nonce: u32 },

    /**
       @notice The function is used to update the edge data of a LinkableAnchor
       @param src_chain_id The chain ID of the chain whose edge needs updating
       @param root The merkle root of the linked anchor on the  `src_chain_id`'s chain
       @param latest_leaf_id The index of the leaf updating the merkle tree with root `root`
    */
    UpdateEdge {
        src_chain_id: u64,
        root: [u8; 32],
        latest_leaf_id: u32,
        target: [u8; 32],
    },

    /**
       @notice Sets the minimal withdrawal limit for the anchor
       @param _minimalWithdrawalAmount The new minimal withdrawal limit
    */
    ConfigureMinimalWithdrawalLimit { minimal_withdrawal_amount: Uint128 },

    /**
       @notice Sets the maximal deposit limit for the anchor
       @param _maximumDepositAmount The new maximal deposit limit
    */
    ConfigureMaximumDepositLimit { maximum_deposit_amount: Uint128 },
}
