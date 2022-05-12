<h1 align="center">Webb Protocol CosmWasm</h1>

<p align="center">
    <strong>🕸️  Webb Protocol CosmWasm!  ⧫</strong>
    <br />
    <sub> ⚠️ Beta Software ⚠️ </sub>
</p>

<br />

## Introduction  
This repository contains the Cosmwasm implementation of **Webb Protocol**, which would be used for *Cosmos SDK* blockchains.   

## Contracts layout  
```
contracts/
    |___anchor/                    # Anchor(FixedDepositAnchor) contract
    |___anchor-handler/            # Contract for executing the creation & modification of anchor  
    |___mixer/                     # Mixer contract  
    |___signature-bridge/          # Contract for managing voting, resource, and maintainer composition through signature verification    
    |___tokenwrapper/              # Contract for wrapping pooled assets and minting pool share tokens  
    |___tokenwrapper-handler/      # Contract for executing the creation & modification of token-wrapper  
    |___vanchor/                   # Variable Anchor contract  
```  

## Building the contracts(wasm)

### Prerequisites
#### Install Rust & dependency
Install the latest version of Rust by following the instructions [here](https://www.rust-lang.org/tools/install).  
Add the compilation target.
```
rustup default stable  
rustup target add wasm32-unknown-unknown
```

### Building
To build the contract, run the following command.
```
cargo wasm
```
You can see the output wasm file in the **target/wasm32-unknown-unknown/release** directory.

For the optimization, run the following command.  
**Important**: You will need [docker](https://www.docker.com/) installed to run this command.  
```
docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/workspace-optimizer:0.12.5
```
Then, you can see the output wasm file in the **artifacts** directory.

## Testing 
Run the following command to run the unit tests.  
```
cargo test --release
```

## License

<sup>
Licensed under <a href="LICENSE">Apache License 2.0</a>.
</sup>

<br/>

<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the GPLV3 license, shall
be licensed as above, without any additional terms or conditions.
</sub>