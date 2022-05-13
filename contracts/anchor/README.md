# comswasm_anchor(FixedDepositAnchor)

This is the cosmwasm implementation of "anchor" contract.

The **FixedDepositAnchor** system is an interoperable shielded pool supporting   
fixed denomination deposits of CW20 tokens.  

The system is to be linked to other *FixedDepositAnchor*s through a simple   
graph-like interface where anchors maintain edges of their neighboring anchors.    

The system requires users to both deposit a fixed denomination of CW20 assets   
into the smart contract and insert a commitment into the underlying merkle tree   
of the form:    
```
commitment = Poseidon(destinationChainId, nullifier, secret)
```  

Commitments adhering to different hash functions and formats will invalidate  
any attempt at withdrawal.  

Information regarding the commitments:   
	- **Poseidon** is a zkSNARK friendly hash function  
	- **destinationChainId** is the chainId of the destination chain, where   
		the withdrawal is intended to be made   
	- **nullifier** is a random field element and identifier for the deposit   
		that will be used to withdraw the deposit and ensure that the deposit   
		is not double withdrawn.  
	- **secret** is a random field element that will remain secret throughout   
	 	the lifetime of the deposit and withdrawal.   

Using the preimage of the commitment, users can generate a zkSNARK proof that  
the deposit is located in one-of-many anchor merkle trees and that the commitment's  
destination chain id matches the underlying chain id of the anchor where the  
withdrawal is taking place. The chain id opcode is leveraged to prevent any  
tampering of this data.  


**NOTE**: For more information, please check the **Webb Protocol** documentation.    
