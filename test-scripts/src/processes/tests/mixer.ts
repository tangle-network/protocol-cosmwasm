import { SigningCosmWasmClient } from "@cosmjs/cosmwasm-stargate";
import { Coin, coin, DirectSecp256k1HdWallet } from "@cosmjs/proto-signing";
import { GasPrice } from "@cosmjs/stargate";
import * as chai from "chai";
import chaiAsPromised from "chai-as-promised";
import chalk from "chalk";

import { localjuno } from "../../config/localjunoConstants";
import { datetimeStringToUTC, toEncodedBinary } from "../../utils/helpers";

import { JsNoteBuilder, MTBn254X5, setupKeys, verify_js_proof } from '../../utils/wasm-utils/njs/wasm-utils-njs';
import { hexToU8a, u8aToHex } from '@polkadot/util';

chai.use(chaiAsPromised);
const { expect } = chai;

// -----------------------------------------------
//  TEST: Mixer
//  
//  SCENARIO: 
//   1. Initialize the "Mixer" contract (already done in "setup")
//   2. Check if the state/config matches the setup input
// ------------------------------------------------
export async function testMixerInitialize(
  junod: SigningCosmWasmClient,
  mixer: string,
): Promise<void> {
  process.stdout.write("Test - Mixer should initialize");
  const result: any = await junod.queryContractSmart(mixer, {
    config: {},
  });

  expect(result.native_token_denom == localjuno.contractsConsts.nativeTokenDenom);
  expect(result.cw20_address == "");
  expect(result.deposit_size == localjuno.contractsConsts.depositSize);

  console.log(chalk.green(" Passed!"));
}

// -----------------------------------------------
//  TEST: Mixer
//  
//  SCENARIO: 
//   1. Wallet3 deposit the "ucosm" tokens to mixer
// ------------------------------------------------
export async function testMixerDepositNativeToken(
  junod: SigningCosmWasmClient,
  mixer: string,
  wallet3: DirectSecp256k1HdWallet,
  ucosm_amount: string,
): Promise<void> {
  process.stdout.write(`Test - Wallet3 deposit ${ucosm_amount} ucosm to mixer`);

  let wallet3_client = await SigningCosmWasmClient.connectWithSigner(
    localjuno.networkInfo.url,
    wallet3,
    { gasPrice: GasPrice.fromString("0.1ujunox") },
  );

  // Fail to "deposit" since no "commitment"
  await expect(
    wallet3_client.execute(localjuno.addresses.wallet3, mixer, {
      deposit: {
        commitment: undefined,
      },
    }, "auto", undefined, [coin(ucosm_amount, "ucosm")])
  ).to.be.rejected; // rejectedWith("Commitment not found");

  // Succeed to "deposit"
  const result = await wallet3_client.execute(localjuno.addresses.wallet3, mixer, {
    deposit: {
      commitment: [60, 193, 57, 161, 207, 107, 11, 192, 51, 187, 64, 70, 168, 216, 155, 216, 187, 112, 123, 6, 14, 101, 174, 89, 250, 120, 41, 24, 101, 151, 110, 24],
    }
  }, "auto", undefined, [coin(ucosm_amount, "ucosm")]);

  console.log(chalk.green(" Passed!"));
}

// -----------------------------------------------
//  TEST: Mixer
//  
//  SCENARIO: 
//   1. Wallet2 withdraw the "ucosm" tokens to mixer
// ------------------------------------------------
export async function testMixerWithdrawNativeToken(
  junod: SigningCosmWasmClient,
  mixer: string,
  wallet1: DirectSecp256k1HdWallet,
  wallet2: DirectSecp256k1HdWallet,
  wallet3: DirectSecp256k1HdWallet,
  ucosm_amount: string,
): Promise<void> {
  process.stdout.write(`Test - Wallet2 withdraw ${ucosm_amount} ucosm from mixer`);

  let wallet2_client = await SigningCosmWasmClient.connectWithSigner(
    localjuno.networkInfo.url,
    wallet2,
    { gasPrice: GasPrice.fromString("0.1ujunox") },
  );

  // Fail to "withdraw" since no "commitment"
  await expect(
    wallet2_client.execute(localjuno.addresses.wallet2, mixer, {
      withdraw: {
        proof_bytes: [171, 78, 91, 39, 195, 136, 25, 239, 54, 52, 122, 184, 250, 174, 86, 201, 15, 212, 162, 6, 172, 35, 88, 216, 105, 141, 206, 241, 161, 143, 106, 33, 110, 194, 247, 183, 7, 179, 197, 11, 117, 153, 201, 44, 24, 204, 171, 120, 246, 61, 240, 100, 230, 5, 56, 207, 143, 160, 180, 20, 66, 164, 183, 29, 228, 215, 232, 241, 176, 233, 48, 1, 230, 80, 81, 75, 124, 187, 249, 143, 42, 251, 94, 129, 130, 135, 11, 188, 129, 79, 246, 70, 154, 79, 154, 131, 54, 121, 242, 112, 167, 81, 122, 180, 61, 115, 248, 65, 96, 62, 87, 21, 42, 108, 237, 81, 181, 163, 129, 56, 124, 89, 206, 139, 62, 230, 16, 37],
        root: undefined,
        nullifier_hash: [183, 160, 141, 89, 98, 241, 220, 87, 120, 249, 242, 56, 92, 41, 28, 230, 247, 111, 155, 7, 94, 2, 142, 101, 0, 243, 39, 32, 59, 235, 198, 31],
        recipient: localjuno.addresses.wallet2,
        relayer: localjuno.addresses.wallet3,
        fee: "0",
        refund: "0",
        cw20_address: undefined,
      },
    }, "auto", undefined, [coin(ucosm_amount, "ucosm")])
  ).to.be.rejected; // rejectedWith("Root is not known");

  // Succeed to "withdraw" 
  const beforeBalance: Coin = await junod.getBalance(localjuno.addresses.wallet2, "ucosm");
  const beforeUcosm = beforeBalance.amount;

  const result = await wallet2_client.execute(localjuno.addresses.wallet2, mixer, {
    withdraw: {
      proof_bytes: [171, 78, 91, 39, 195, 136, 25, 239, 54, 52, 122, 184, 250, 174, 86, 201, 15, 212, 162, 6, 172, 35, 88, 216, 105, 141, 206, 241, 161, 143, 106, 33, 110, 194, 247, 183, 7, 179, 197, 11, 117, 153, 201, 44, 24, 204, 171, 120, 246, 61, 240, 100, 230, 5, 56, 207, 143, 160, 180, 20, 66, 164, 183, 29, 228, 215, 232, 241, 176, 233, 48, 1, 230, 80, 81, 75, 124, 187, 249, 143, 42, 251, 94, 129, 130, 135, 11, 188, 129, 79, 246, 70, 154, 79, 154, 131, 54, 121, 242, 112, 167, 81, 122, 180, 61, 115, 248, 65, 96, 62, 87, 21, 42, 108, 237, 81, 181, 163, 129, 56, 124, 89, 206, 139, 62, 230, 16, 37],
      root: [82, 25, 2, 85, 65, 173, 18, 5, 74, 175, 108, 14, 232, 197, 174, 9, 242, 59, 105, 48, 104, 169, 204, 128, 253, 150, 15, 102, 108, 214, 81, 33],
      nullifier_hash: [183, 160, 141, 89, 98, 241, 220, 87, 120, 249, 242, 56, 92, 41, 28, 230, 247, 111, 155, 7, 94, 2, 142, 101, 0, 243, 39, 32, 59, 235, 198, 31],
      recipient: localjuno.addresses.wallet2,
      relayer: localjuno.addresses.wallet3,
      fee: "0",
      refund: "0",
      cw20_address: undefined,
    },
  }, "auto", undefined, [coin(ucosm_amount, "ucosm")]);

  const afterBalance: Coin = await junod.getBalance(localjuno.addresses.wallet2, "ucosm");
  const afterUcosm = afterBalance.amount;

  expect(parseInt(beforeUcosm) + parseInt(ucosm_amount) == parseInt(afterUcosm));

  console.log(chalk.green(" Passed!"));
}

