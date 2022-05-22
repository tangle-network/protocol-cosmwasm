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
): Promise<void> {
  console.log(chalk.yellow("\nStep 3. Running Tests"));
  // SignatureBridge

  // TokenWrapper
  await testTokenWrapperInitialize(junod, tokenWrapper);

  // TokenWrapperHandler

  // AnchorHandler

  // Anchor

  // VAnchor

  // // Mixer
  await testMixerInitialize(junod, mixer);
  // await testMixerDepositNativeToken(junod, mixer, wallet3, "100");
  // await testMixerWithdrawNativeToken(junod, mixer, wallet1, wallet2, wallet3, "100");
  
  process.exit();
}


// -----------------------------------------------
//  TEST: TokenWrapper
//  
//  SCENARIO: 
//   1. Initialize the "(Governed)TokenWrapper" contract (already done in "setup")
//   2. Check if the state/config matches the setup input
// ------------------------------------------------
async function testTokenWrapperInitialize(
  junod: SigningCosmWasmClient,
  tokenWrapper: string,
): Promise<void> {
  process.stdout.write("Test - TokenWrapper should initialize");
  const result: any = await junod.queryContractSmart(tokenWrapper, {
    config: {},
  });
  // console.log(result);

  expect(result.governor == localjuno.addresses.wallet1).to.be.ok;
  expect(result.native_token_denom == localjuno.contractsConsts.nativeTokenDenom).to.be.ok;
  expect(result.fee_recipient == localjuno.addresses.wallet2).to.be.ok;
  expect(result.fee_percentage == (parseInt(localjuno.contractsConsts.feePercentage) / 100).toString()).to.be.ok;
  expect(result.is_native_allowed == (localjuno.contractsConsts.isNativeAllowed == 1).toString()).to.be.ok;
  expect(result.wrapping_limit == localjuno.contractsConsts.tokenWrapperWrappingLimit).to.be.ok;
  expect(result.proposal_nonce == "0").to.be.ok;
  
  console.log(chalk.green(" Passed!"));
}


// -----------------------------------------------
//  TEST: Mixer
//  
//  SCENARIO: 
//   1. Initialize the "Mixer" contract (already done in "setup")
//   2. Check if the state/config matches the setup input
// ------------------------------------------------
async function testMixerInitialize(
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
async function testMixerDepositNativeToken(
  junod: SigningCosmWasmClient,
  mixer: string,
  wallet3: DirectSecp256k1HdWallet,
  ucosm_amount: string,
): Promise<void> {
  process.stdout.write(`Test - Wallet3 deposit ${ucosm_amount} ucosm to mixer`);

  let wallet3_client = await SigningCosmWasmClient.connectWithSigner(
    localjuno.networkInfo.url,
    wallet3,
    {gasPrice: GasPrice.fromString("0.1ujunox")},
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
  console.log(result);

  console.log(chalk.green(" Passed!"));
}

// -----------------------------------------------
//  TEST: Mixer
//  
//  SCENARIO: 
//   1. Wallet2 withdraw the "ucosm" tokens to mixer
// ------------------------------------------------
async function testMixerWithdrawNativeToken(
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
    {gasPrice: GasPrice.fromString("0.1ujunox")},
  );

  // Fail to "withdraw" since no "commitment"
  await expect(
    wallet2_client.execute(localjuno.addresses.wallet2, mixer, {
      withdraw: {
        proof_bytes: [229, 214, 117, 134, 217, 67, 12, 236, 196, 111, 110, 244, 116, 12, 30, 219, 27, 206, 151, 233, 126, 189, 160, 237, 55, 126, 47, 5, 16, 214, 38, 40, 73, 190, 123, 2, 2, 209, 193, 209, 130, 242, 27, 207, 132, 223, 159, 121, 241, 109, 55, 190, 251, 72, 255, 132, 221, 100, 139, 132, 94, 57, 26, 3, 127, 190, 105, 168, 228, 222, 91, 22, 209, 99, 227, 6, 130, 238, 109, 47, 20, 85, 125, 67, 77, 26, 176, 24, 95, 6, 159, 150, 5, 229, 254, 144, 188, 203, 207, 201, 167, 255, 5, 93, 210, 27, 38, 151, 73, 234, 247, 124, 71, 103, 23, 101, 83, 90, 109, 120, 10, 58, 150, 8, 211, 218, 219, 155],
        root: [80, 25, 2, 85, 65, 173, 18, 5, 74, 175, 108, 14, 232, 197, 174, 9, 242, 59, 105, 48, 104, 169, 204, 128, 253, 150, 15, 102, 108, 214, 81, 33],
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
      proof_bytes: [229, 214, 117, 134, 217, 67, 12, 236, 196, 111, 110, 244, 116, 12, 30, 219, 27, 206, 151, 233, 126, 189, 160, 237, 55, 126, 47, 5, 16, 214, 38, 40, 73, 190, 123, 2, 2, 209, 193, 209, 130, 242, 27, 207, 132, 223, 159, 121, 241, 109, 55, 190, 251, 72, 255, 132, 221, 100, 139, 132, 94, 57, 26, 3, 127, 190, 105, 168, 228, 222, 91, 22, 209, 99, 227, 6, 130, 238, 109, 47, 20, 85, 125, 67, 77, 26, 176, 24, 95, 6, 159, 150, 5, 229, 254, 144, 188, 203, 207, 201, 167, 255, 5, 93, 210, 27, 38, 151, 73, 234, 247, 124, 71, 103, 23, 101, 83, 90, 109, 120, 10, 58, 150, 8, 211, 218, 219, 155],
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
