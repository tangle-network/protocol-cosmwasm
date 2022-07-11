import { SigningCosmWasmClient } from "@cosmjs/cosmwasm-stargate";
import { Coin, coin, DirectSecp256k1HdWallet } from "@cosmjs/proto-signing";
import { GasPrice } from "@cosmjs/stargate";
import * as chai from "chai";
import chaiAsPromised from "chai-as-promised";
import chalk from "chalk";
import { localjuno } from "../../config/localjunoConstants";
import { datetimeStringToUTC,toEncodedBinary } from "../../utils/helpers";

chai.use(chaiAsPromised);
const { expect } = chai;

import {
  testTokenWrapperInitialize
} from './tokenWrapper';

import {
  testAnchorInitialize,
  testAnchorDepositWithdraw,
  testAnchorWrapNative,
  testAnchorUnwrapNative,
  testAnchorWrapCw20,
  testAnchorUnwrapCw20,
} from './anchor';

import {
  testVAnchorInitialize,
  testVAnchorDepositWithdraw,
  testVAnchorWrapNative,
  testVAnchorUnwrapNative,
  testVAnchorWrapCw20,
  testVAnchorUnwrapCw20,
} from './vanchor';

import {
  testMixerInitialize,
  testMixerDepositNativeToken,
  testMixerWithdrawNativeToken,
} from './mixer';

import { 
  testSignatureBridgeAdminSetResWithSignature,
  testSignatureBridgeExecProposalWithSignature,
  testSignatureBridgeInitialize
} from "./signatureBridge";

import { testTreasuryInitialize } from "./treasury";

export async function testExecute(
  junod: SigningCosmWasmClient,
  wallet1: DirectSecp256k1HdWallet,
  wallet2: DirectSecp256k1HdWallet,
  wallet3: DirectSecp256k1HdWallet,
  cw20: string,
  signatureBridge: string,
  tokenWrapper: string,
  tokenWrapperHandler: string,
  anchorHandler: string,
  anchor: string,
  vanchor: string,
  mixer: string,
  treasury: string,
  treasuryHandler: string,
): Promise<void> {
  console.log(chalk.yellow("\nStep 3. Running Tests"));
  // // SignatureBridge
  // await testSignatureBridgeInitialize(junod, signatureBridge);
  // await testSignatureBridgeAdminSetResWithSignature(junod, wallet1, signatureBridge);
  // await testSignatureBridgeExecProposalWithSignature(junod, wallet1, signatureBridge);

  // // TokenWrapper
  // await testTokenWrapperInitialize(junod, tokenWrapper);

  // // TokenWrapperHandler

  // // AnchorHandler

  // // Anchor
  // await testAnchorInitialize(junod, anchor);
  // await testAnchorDepositWithdraw(junod, anchor, wallet1, wallet2, wallet3, "1000000");
  // await testAnchorWrapNative(junod, anchor, wallet3, "100000");
  // await testAnchorUnwrapNative(junod, anchor, wallet3, "100");
  // await testAnchorWrapCw20(junod, anchor, tokenWrapper, cw20, wallet3, "10000");
  // await testAnchorUnwrapCw20(junod, anchor, tokenWrapper, cw20, wallet3, "100");

  // // VAnchor
  // await testVAnchorInitialize(junod, vanchor);
  // await testVAnchorDepositWithdraw(junod, vanchor, cw20, wallet1, wallet2, wallet3, "10", "10", "0", "-7", "-5", "2");
  // await testVAnchorWrapNative(junod, vanchor, wallet3, "100000");
  // await testVAnchorUnwrapNative(junod, vanchor, wallet3, "100");
  // await testVAnchorWrapCw20(junod, vanchor, tokenWrapper, cw20, wallet3, "10000");
  // await testVAnchorUnwrapCw20(junod, vanchor, tokenWrapper, cw20, wallet3, "100");

  // // Mixer
  // await testMixerInitialize(junod, mixer);
  // await testMixerDepositNativeToken(junod, mixer, wallet3, "1000000");
  // await testMixerWithdrawNativeToken(junod, mixer, wallet1, wallet2, wallet3, "1000000");

  // Treasury
  await testTreasuryInitialize(junod, treasury);
  
  process.exit();
}
