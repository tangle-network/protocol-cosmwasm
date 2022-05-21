# Webb Protocol-cosmwasm Integration Tests(Cosmjs + LocalJuno)

## Requirements

- docker
- [LocalJuno](https://github.com/CosmosContracts/juno)

## Procedures

### Start LocalJuno

```bash
git clone https://github.com/CosmosContracts/juno.git
cd juno
```

Once done, start LocalJuno by

```bash
STAKE_TOKEN=ujunox UNSAFE_CORS=true docker-compose up  # Ctrl + C to quit
```
(For the detail, please reference [Juno Doc](https://docs.junonetwork.io/smart-contracts-and-junod-development/junod-local-dev-setup#quick-est-start-dev-build))

When you may need to revert LocalJuno to its initial state, run

```bash
docker-compose rm
```

### Compile contracts

```bash
# .zshrc or .bashrc
# set the optimizer version to whichever latest version of optimizer (currently it is 0.12.5):
alias workspace-optimizer='docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/workspace-optimizer:0.12.5'
```

```bash
# from the root folder in the autonomy-cosmwasm-program repo
workspace-optimizer
```

### Create and configure wasms paths file

You need to tell the test suite where to find the wasms artifacts files locally for the various repos it works with.


First, copy the built wasm files into the `./src/config/wasms` dir of this repo.

In the `src/config` folder there is an example file for setting the parameters that point to your local wasm folders: `wasmPaths.ts.example`
In the newly created file, edit the `wasm_path` object's attributes for the `station` to point to the `./src/config/wasms` dir.

```bash
cp ./src/config/wasmPaths.ts.example ./src/config/wasmPaths.ts
nano ./src/config/wasmPaths.ts
```

### LocalJuno constants file setup

In the `src/config` folder there is an example file for setting the constants for your LocalJuno parameters (contracts, wallets, etc): `localjunoConstants.ts.example`

```bash
cp ./src/config/localjunoConstants.ts.example ./src/config/localjunoConstants.ts
nano ./src/config/localjunoConstants.ts
```

### Run full setup of contracts & all tests

```bash
yarn
yarn test:localjuno-setup-station
yarn test:localjuno-tests
```

**NOTE:** After each of the setup commands, you may see key contract addresses or wasm codes that will need to updated in your `localjunoConstatns.ts` file before proceeding to run the next command. These commands build upon on another.  
Also, after one command, the terminal does not automatically get back. So, you should do it manually by `Ctrl + C`.
