# cosmwasm-tokenwrapper(governed)

This is the cosmwasm implementation of "GovernedTokenWrapper" contract.

- A governed TokenWrapper system using an external `governor` address  

- Governs allowable CW20s to deposit using a governable wrapping limit and
  sets fees for wrapping into itself. This contract is intended to be used with
  TokenHandler contract.