# cosmwasm-tokenwrapper(governed)

This is the cosmwasm implementation of "GovernedTokenWrapper" contract.

Basically, this contract is a CW20 token that allows other CW20s to wrap into and mint it.

This contract is intended to be used with **TokenHandler** contract.

This contract also has a **governance** functionality.

- **Governed TokenWrapper** system using an external `governor` address  

- Governs allowable CW20s to deposit using a *governable wrapping limit* and
  sets *fee*s for wrapping into itself.