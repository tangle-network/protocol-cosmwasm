# cosmwasm-vanchor

## Variable Anchor contract  

The Variable Anchor is a variable-denominated shielded pool system  
derived from Tornado Nova (tornado-pool). This system extends the shielded   
pool system into a bridged system and allows for join/split transactions.  
The system is built on top the Anchor/LinkableTree system which allows  
it to be linked to other VAnchor contracts through a simple graph-like  
interface where anchors maintain edges of their neighboring anchors.  
The system requires users to create and deposit UTXOs for the supported CW20  
asset into the smart contract and insert a commitment into the underlying  
merkle tree of the form: commitment = Poseidon(chainID, amount, pubKey, blinding).  
The hash input is the UTXO data. All deposits/withdrawals are unified under  
a common `transact` function which requires a zkSNARK proof that the UTXO commitments  
are well-formed (i.e. that the deposit amount matches the sum of new UTXOs' amounts).  

Information regarding the commitments:  
- Poseidon is a zkSNARK friendly hash function  
- destinationChainID is the chainId of the destination chain, where the withdrawal  
    is intended to be made  
- Details of the UTXO and hashes are below  
UTXO = { destinationChainID, amount, pubkey, blinding }  
commitment = Poseidon(destinationChainID, amount, pubKey, blinding)  
nullifier = Poseidon(commitment, merklePath, sign(privKey, commitment, merklePath))  
Commitments adhering to different hash functions and formats will invalidate  
any attempt at withdrawal.  

Using the preimage / UTXO of the commitment, users can generate a zkSNARK proof that  
the UTXO is located in one-of-many VAnchor merkle trees and that the commitment's  
destination chain id matches the underlying chain id of the VAnchor where the  
transaction is taking place. The chain id opcode is leveraged to prevent any  
tampering of this data.  

## What's special in cosmwasm-vanchor  

Normally, all deposits/withdrawals are unified under a common `transact` function in `vanchor` contract.  
However, `cosmwasm-vanchor` introduces 2 entries: `transact-deposit(receive_cw20)` and `transact-withdraw`.   
  
In `cw20` token standard, if the user wants to trigger action on target contract with token transfer, she  
should use the `Send{contract, amount, msg}` message.  
The target contract should implement `Receive{sender, amount, msg}` message.
Naturally, these messages do not allow zero amount in their `amount` field from the obvious reasons.

`transact-deposit(receive_cw20)` function must be implemented using the `Send/Receive` mechanism,   
since it needs the `cw20` token transfer first & tx execution next.

But, `transact-withdraw` does not need to be using `Send/Receive` mechanism, since there is no `cw20`  
token transfer before tx execution. Also, if we use `Send/Receive` mechanism for `transact-withdraw`,  
it always runs into error, since `amount` is always zero.