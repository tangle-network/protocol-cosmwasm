# cosmwasm-mixer
Implement the (ink) mixer in cosmwasm
## How to build & test the "cosmwasm-mixer"

NOTE: All of the following procedures are based on assumption that the user is
using the Linux(Ubuntu) system or MacOS on the local machine.
If you are using another OS, please reference the following URL.
https://docs.terra.money/docs/develop/dapp/smart-contracts/write-smart-contract.html#building-the-contract

### Prerequisites

    - Install Rust & its utils
        Add the compilation target.
        ```
        rustup default stable
        rustup target add wasm32-unknown-unknown
        ```

        Install the utils
        ```
        cargo install cargo-generate --features vendored-openssl
        cargo install cargo-run-script
        ```

### Build the contract

    - To build the contract, run the following command.
        ```
        cargo wasm
        ```
        You can see the output wasm file in the "target/wasm32-unknown-unknown/release" dir.

    - For the optimization, run the following command.
        ```
        cargo run-script optimize
        ```
        Then, you can see the output wasm file in the "artifacts" dir.

### Run the unit tests

    - For running the unit tests(written on "src/contract.rs"), run the following command.
        ```
        cargo test
        ```
        
