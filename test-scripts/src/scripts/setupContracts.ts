import { 
    LCDClient, 
    MnemonicKey, 
    Wallet, 
} from "@terra-money/terra.js";
import chalk from "chalk";
import config from "../config";
import { storeCode, instantiateContract, } from '../utils';

// Variables
let terra: LCDClient;
let deployer: Wallet;

let mixer: string;
let anchor: string;
let vanchor: string;
let cw20: string;

function initialize() {
  terra = new LCDClient({
      URL: config.networkInfo.lcd_url,
      chainID: config.networkInfo.chainId,
      gasPrices: { uluna: config.networkInfo.gasPrice },
      gasAdjustment: config.networkInfo.gasAdjustment,
  });
    deployer = terra.wallet(new MnemonicKey({ mnemonic: config.mnemonicKeys.deployer }));

    console.log(`Use ${chalk.cyan(deployer.key.accAddress)} as wallet(deployer)`);

    mixer = config.contracts.mixer;
    anchor = config.contracts.anchor;
    vanchor = config.contracts.vanchor;
    cw20 = config.contracts.cw20;

    console.log(`Use ${chalk.cyan(cw20)} as Cw20 token contract`);
    console.log(`Use ${chalk.cyan(mixer)} as Mixer`);
    console.log(`Use ${chalk.cyan(anchor)} as Anchor`);
    console.log(`Use ${chalk.cyan(vanchor)} as Vanchor`);

}

export async function setupMixer(): Promise<void> {
    // Initialize environment info
    console.log("1. Setup Environment");
    initialize();

    // Setup mixer
    console.log("2. Setup mixer");

    // upload mixer wasm
    const mixerCodeId = await storeCode(terra, deployer, "cosmwasm_mixer.wasm");
    console.log(chalk.green("Done!", `${chalk.blue("codeId")} = ${mixerCodeId}`));

    // instantiate mixer
    const mixerResult = await instantiateContract(
        terra, 
        deployer, 
        deployer, 
        mixerCodeId, 
        {
            "deposit_size": "1000000", 
            "merkletree_levels": 30, 
            "cw20_address": undefined, 
            "native_token_denom": "uusd",
        }
    );
    mixer = mixerResult.logs[0].events
        .find((event) => {
        return event.type == "instantiate_contract";
        })
        ?.attributes.find((attribute) => {
        return attribute.key == "contract_address";
        })?.value as string;
    console.log(chalk.green("Done!"), `${chalk.blue("contractAddress")} = ${mixer}`);
}

export async function setupAnchor(): Promise<void> {
    // Initialize environment info
    console.log("1. Setup Environment");
    initialize();

    // Setup anchor
    console.log("2. Setup Anchor");

  // upload anchor wasm
  const anchorCodeId = await storeCode(terra, deployer, "cosmwasm_anchor.wasm");
  console.log(chalk.green("Done!", `${chalk.blue("codeId")} = ${anchorCodeId}`));

  // instantiate anchor
  const anchorResult = await instantiateContract(
      terra, 
      deployer, 
      deployer, 
      anchorCodeId, 
      {
          "max_edges": 2, 
          "chain_id": 1,
          "levels": 30,
          "deposit_size": "1000000", 
          "cw20_address": cw20, 
      }
  );
  anchor = anchorResult.logs[0].events
    .find((event) => {
      return event.type == "instantiate_contract";
    })
    ?.attributes.find((attribute) => {
      return attribute.key == "contract_address";
    })?.value as string;
  console.log(chalk.green("Done!"), `${chalk.blue("contractAddress")}=${anchor}`);
}
