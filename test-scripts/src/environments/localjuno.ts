// -------------------------------------------------------------------------------------
// LocalJuno test-suite
// -------------------------------------------------------------------------------------
import chalk from "chalk";
import { GasPrice } from "@cosmjs/stargate";
import { DirectSecp256k1HdWallet } from "@cosmjs/proto-signing";
import { SigningCosmWasmClient } from "@cosmjs/cosmwasm-stargate";

import { localjuno as config, localjuno } from "../config/localjunoConstants";
import { datetimeStringToUTC } from "../utils/helpers";

import { migrateContracts } from "../processes/migrate/testnet";
import { setupContracts } from "../processes/setup/testnet";

import { testExecute } from "../processes/tests/testnet";

// -------------------------------------------------------------------------------------
// Variables
// -------------------------------------------------------------------------------------
let junod: SigningCosmWasmClient;
let wallet1: DirectSecp256k1HdWallet;
let wallet2: DirectSecp256k1HdWallet;
let wallet3: DirectSecp256k1HdWallet;

let cw20: string;
let signatureBridge: string;
let tokenWrapper: string;
let tokenWrapperHandler: string;
let anchorHandler: string;
let anchor: string;
let vanchor: string;
let mixer: string;
let treasury: string;
let treasuryHandler: string;

// -------------------------------------------------------------------------------------
// initialize variables
// -------------------------------------------------------------------------------------
async function initialize() {
    wallet1 = await DirectSecp256k1HdWallet.fromMnemonic(localjuno.mnemonicKeys.wallet1, { prefix: "juno" });
    wallet2 = await DirectSecp256k1HdWallet.fromMnemonic(localjuno.mnemonicKeys.wallet2, { prefix: "juno" });
    wallet3 = await DirectSecp256k1HdWallet.fromMnemonic(localjuno.mnemonicKeys.wallet3, { prefix: "juno" });

    junod = await SigningCosmWasmClient.connectWithSigner(localjuno.networkInfo.url, wallet1, { gasPrice: GasPrice.fromString("0.1ujunox")});

    const [account1] = await wallet1.getAccounts();
    const [account2] = await wallet2.getAccounts();
    const [account3] = await wallet3.getAccounts();

    console.log(`Use ${chalk.cyan(account1.address)} as Wallet 1`);
    console.log(`Use ${chalk.cyan(account2.address)} as Wallet 2`);
    console.log(`Use ${chalk.cyan(account3.address)} as Wallet 3`);

    cw20 = config.contracts.cw20;
    signatureBridge = config.contracts.signatureBridge;
    tokenWrapper = config.contracts.tokenWrapper;
    tokenWrapperHandler = config.contracts.tokenWrapperHandler;
    anchorHandler = config.contracts.anchorHandler;
    anchor = config.contracts.anchor;
    vanchor = config.contracts.vanchor;
    mixer = config.contracts.mixer;
    treasury = config.contracts.treasury;
    treasuryHandler = config.contracts.treasuryHandler;

    console.log(`Use ${chalk.cyan(cw20)} as Cw20(AUTO) token`);
    console.log(`Use ${chalk.cyan(signatureBridge)} as SignatureBridge`);
    console.log(`Use ${chalk.cyan(tokenWrapper)} as TokenWrapper`);
    console.log(`Use ${chalk.cyan(tokenWrapperHandler)} as TokenWrapperHandler`);
    console.log(`Use ${chalk.cyan(anchorHandler)} as AnchorHandler`);
    console.log(`Use ${chalk.cyan(anchor)} as Anchor`);
    console.log(`Use ${chalk.cyan(vanchor)} as VAnchor`);
    console.log(`Use ${chalk.cyan(mixer)} as Mixer`);
    console.log(`Use ${chalk.cyan(treasury)} as Treasury`);
    console.log(`Use ${chalk.cyan(treasuryHandler)} as TreasuryHandler`);
}


// -------------------------------------------------------------------------------------
// setup contracts
// -------------------------------------------------------------------------------------
export async function startSetupContracts(): Promise<void> {
    console.log(chalk.blue("\nTestNet"));

    // Initialize environment information
    console.log(chalk.yellow("\nStep 1. Environment Info"));
    await initialize();

    // Setup contracts
    console.log(chalk.yellow("\nStep 2. Contracts Setup"));
    await setupContracts(junod, { wallet1, wallet2, wallet3 });
}


// -------------------------------------------------------------------------------------
// start test
// -------------------------------------------------------------------------------------
export async function startTests(): Promise<void> {
    console.log(chalk.blue("\nTestNet"));
  
    // Initialize environment information
    console.log(chalk.yellow("\nStep 1. Environment Info"));
    await initialize();
  
    // Test queries
    await testExecute(
        junod,
        wallet1,
        wallet2,
        wallet3,
        cw20,
        signatureBridge,
        tokenWrapper,
        tokenWrapperHandler,
        anchorHandler,
        anchor,
        vanchor,
        mixer,
        treasury, 
        treasuryHandler
    );
}
