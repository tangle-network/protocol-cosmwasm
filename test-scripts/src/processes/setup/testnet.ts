/* eslint-disable @typescript-eslint/no-explicit-any */
import * as path from "path";
import chalk from "chalk";
import { storeCode, instantiateContract } from "../../utils/helpers";
import { wasm_path } from "../../config/wasmPaths";
import { NONAME } from "dns";

import { SigningCosmWasmClient } from "@cosmjs/cosmwasm-stargate";
import { coin, DirectSecp256k1HdWallet } from "@cosmjs/proto-signing";

import { localjuno } from '../../config/localjunoConstants';

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

// -------------------------------------------------------------------------------------
// setup all contracts for LocalJuno and TestNet
// -------------------------------------------------------------------------------------
export async function setupContracts(
    junod: SigningCosmWasmClient,
    wallets: {
        wallet1: DirectSecp256k1HdWallet,
        wallet2: DirectSecp256k1HdWallet,
        wallet3: DirectSecp256k1HdWallet,
    }
): Promise<void> {
    junod = junod;
    wallet1 = wallets.wallet1;
    wallet2 = wallets.wallet2;
    wallet3 = wallets.wallet3;

    // Send some test tokens to test wallets
    await junod.sendTokens(localjuno.addresses.wallet1, localjuno.addresses.wallet2, [coin("100000000", "ucosm"), coin("10000000", "ujunox")], "auto");
    await junod.sendTokens(localjuno.addresses.wallet1, localjuno.addresses.wallet3, [coin("100000000", "ucosm"), coin("10000000", "ujunox")], "auto");

    await setup(junod, wallet1);

    console.log(chalk.green(" Done!"));
}

async function setup(
    junod: SigningCosmWasmClient,
    wallet1: DirectSecp256k1HdWallet,
): Promise<void> {
    // Step 1. Upload all local wasm files and capture the codes for each....
    
    process.stdout.write("Uploading CW20 Token Wasm");
    const cw20CodeId = await storeCode(
        junod,
        wallet1, 
        `${wasm_path.station}/cw20_base.wasm`
    );
    console.log(chalk.green(" Done!"), `${chalk.blue("codeId")} = ${cw20CodeId}`);
   
    process.stdout.write("Uploading SignatureBridge Wasm");
    const signatureBridgeCodeId = await storeCode(
        junod, 
        wallet1,
        `${wasm_path.station}/cosmwasm_signature_bridge.wasm`
    );

    console.log(chalk.green(" Done!"), `${chalk.blue("codeId")}=${signatureBridgeCodeId}`);

    process.stdout.write("Uploading TokenWrapper Wasm");
    const tokenWrapperCodeId = await storeCode(
        junod, 
        wallet1,
        `${wasm_path.station}/cosmwasm_tokenwrapper.wasm`
    );

    console.log(chalk.green(" Done!"), `${chalk.blue("codeId")}=${tokenWrapperCodeId}`);

    process.stdout.write("Uploading TokenWrapperHandler Wasm");
    const tokenWrapperHandlerCodeId = await storeCode(
        junod,
        wallet1, 
        `${wasm_path.station}/cosmwasm_tokenwrapper_handler.wasm`
    );
    console.log(chalk.green(" Done!"), `${chalk.blue("codeId")} = ${tokenWrapperHandlerCodeId}`);

    process.stdout.write("Uploading AnchorHandler Wasm");
    const anchorHandlerCodeId = await storeCode(
        junod,
        wallet1, 
        `${wasm_path.station}/cosmwasm_anchor_handler.wasm`
    );
    console.log(chalk.green(" Done!"), `${chalk.blue("codeId")} = ${anchorHandlerCodeId}`);

    process.stdout.write("Uploading Anchor Wasm");
    const anchorCodeId = await storeCode(
        junod,
        wallet1, 
        `${wasm_path.station}/cosmwasm_anchor.wasm`
    );
    console.log(chalk.green(" Done!"), `${chalk.blue("codeId")} = ${anchorCodeId}`);

    process.stdout.write("Uploading VAnchor Wasm");
    const vanchorCodeId = await storeCode(
        junod,
        wallet1, 
        `${wasm_path.station}/cosmwasm_vanchor.wasm`
    );
    console.log(chalk.green(" Done!"), `${chalk.blue("codeId")} = ${vanchorCodeId}`);

    process.stdout.write("Uploading Mixer Wasm");
    const mixerCodeId = await storeCode(
        junod,
        wallet1, 
        `${wasm_path.station}/cosmwasm_mixer.wasm`
    );
    console.log(chalk.green(" Done!"), `${chalk.blue("codeId")} = ${mixerCodeId}`);


    // Step 2. Instantiate contracts

    // CW20 token 
    process.stdout.write("Instantiating CW20(AUTO) token contract");

    const autoTokenResult = await instantiateContract(
        junod,
        wallet1,
        wallet1,
        cw20CodeId,
        {
            "name": localjuno.contractsConsts.cw20TokenName,
            "symbol": localjuno.contractsConsts.cw20TokenSymbol,
            "decimals": localjuno.contractsConsts.decimals,
            "initial_balances": [
                {
                    "address": localjuno.addresses.wallet1,
                    "amount": "10000000000"
                },
                {
                    "address": localjuno.addresses.wallet2,
                    "amount": "10000000000"
                },
                {
                    "address": localjuno.addresses.wallet3,
                    "amount": "10000000000"
                }
            ]   
        }
    );
    cw20 = autoTokenResult.contractAddress;
    console.log(chalk.green(" Done!"), `${chalk.blue("contractAddress")}=${cw20}`);


    // SignatureBridge
    process.stdout.write("Instantiating SignatureBridge contract");
    const [account] = await wallet1.getAccounts();
    const signatureBridgeResult = await instantiateContract(
        junod,
        wallet1,
        wallet1,
        signatureBridgeCodeId,
        {
            "initial_governor": Array.from(account.pubkey),
        }
      );
    signatureBridge = signatureBridgeResult.contractAddress;
    console.log(chalk.green(" Done!"), `${chalk.blue("contractAddress")}=${signatureBridge}`);


    // TokenWrapper
    process.stdout.write("Instantiating tokenWrapper contract");
    
    const tokenWrapperResult = await instantiateContract(
        junod,
        wallet1,
        wallet1,
        tokenWrapperCodeId,
        {
            "name": localjuno.contractsConsts.tokenWrapperTokenName,
            "symbol": localjuno.contractsConsts.tokenWrapperTokenSymbol,
            "decimals": localjuno.contractsConsts.decimals,
            "governor": undefined,
            "fee_recipient": localjuno.addresses.wallet2,
            "fee_percentage": localjuno.contractsConsts.feePercentage,
            "native_token_denom": localjuno.contractsConsts.nativeTokenDenom,
            "is_native_allowed": localjuno.contractsConsts.isNativeAllowed,
            "wrapping_limit": localjuno.contractsConsts.tokenWrapperWrappingLimit,
        }
      );
    tokenWrapper = tokenWrapperResult.contractAddress;

    // Register Cw20(AUTO) token in "TokenWrapper"
    await junod.execute(localjuno.addresses.wallet1, tokenWrapper, {
        add_cw20_token_addr: {
            token: cw20,
            nonce: 1,
        }
    }, "auto", undefined, []);
    
    console.log(chalk.green(" Done!"), `${chalk.blue("contractAddress")}=${tokenWrapper}`);


    // TokenWrapperHandler
    process.stdout.write("Instantiating TokenWrapperHandler contract");
    
    const tokenWrapperHandlerResult = await instantiateContract(
        junod,
        wallet1,
        wallet1,
        tokenWrapperHandlerCodeId,
        {
            "bridge_addr": signatureBridge,
            "initial_resource_ids": [],
            "initial_contract_addresses": [],
        }
      );
    tokenWrapperHandler = tokenWrapperHandlerResult.contractAddress;
    console.log(chalk.green(" Done!"), `${chalk.blue("contractAddress")}=${tokenWrapperHandler}`);


    // AnchorHandler
    process.stdout.write("Instantiating AnchorHandler contract");
    
    const anchorHandlerResult = await instantiateContract(
        junod,
        wallet1,
        wallet1,
        anchorHandlerCodeId,
        {
            "bridge_addr": signatureBridge,
            "initial_resource_ids": [],
            "initial_contract_addresses": [],
        }
      );
    anchorHandler = anchorHandlerResult.contractAddress;
    console.log(chalk.green(" Done!"), `${chalk.blue("contractAddress")}=${anchorHandler}`);


    // Anchor
    process.stdout.write("Instantiating Anchor contract");
    
    const anchorResult = await instantiateContract(
        junod,
        wallet1,
        wallet1,
        anchorCodeId,
        {
            "max_edges": localjuno.contractsConsts.maxEdges,
            "levels": localjuno.contractsConsts.levels,
            "deposit_size": localjuno.contractsConsts.depositSize,
            "tokenwrapper_addr": tokenWrapper,
            "handler": anchorHandler,
        }
      );
    anchor = anchorResult.contractAddress;
    console.log(chalk.green(" Done!"), `${chalk.blue("contractAddress")}=${anchor}`);

   
    // VAnchor
    process.stdout.write("Instantiating VAnchor contract");
    
    const vanchorResult = await instantiateContract(
        junod,
        wallet1,
        wallet1,
        vanchorCodeId,
        {
            "max_edges": localjuno.contractsConsts.maxEdges,
            "levels": localjuno.contractsConsts.levels,
            "max_deposit_amt": localjuno.contractsConsts.maxDepositAmt,
            "min_withdraw_amt": localjuno.contractsConsts.minWithdrawAmt,
            "max_ext_amt": localjuno.contractsConsts.maxExtAmt,
            "max_fee": localjuno.contractsConsts.maxFee,
            "tokenwrapper_addr": tokenWrapper,
            "handler": anchorHandler,
        }
      );
    vanchor = vanchorResult.contractAddress;
    console.log(chalk.green(" Done!"), `${chalk.blue("contractAddress")}=${vanchor}`);     

    // Mixer
    process.stdout.write("Instantiating Mixer contract");
    
    const mixerResult = await instantiateContract(
        junod,
        wallet1,
        wallet1,
        mixerCodeId,
        {
            "merkletree_levels": localjuno.contractsConsts.levels, 
            "deposit_size": localjuno.contractsConsts.depositSize,
            "native_token_denom": localjuno.contractsConsts.nativeTokenDenom,
            "cw20_address": undefined,
        }
      );
    mixer = mixerResult.contractAddress;
    console.log(chalk.green(" Done!"), `${chalk.blue("contractAddress")}=${mixer}`);

    process.exit();
}