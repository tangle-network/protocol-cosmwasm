import { SigningCosmWasmClient } from "@cosmjs/cosmwasm-stargate";
import { Coin, coin, DirectSecp256k1HdWallet } from "@cosmjs/proto-signing";
import { GasPrice } from "@cosmjs/stargate";
import * as chai from "chai";
import chaiAsPromised from "chai-as-promised";
import chalk from "chalk";
import { localjuno } from "../../config/localjunoConstants";
import { datetimeStringToUTC,toEncodedBinary } from "../../utils/helpers";
import { } from "@webb-tools/api";

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
  await testAnchorInitialize(junod, anchor);
  // await testAnchorDepositWithdraw(junod, anchor, wallet1, wallet2, wallet3, "1000000");
  await testAnchorWrapNative(junod, anchor, wallet3, "100000");
  await testAnchorUnwrapNative(junod, anchor, wallet3, "100");

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


// -----------------------------------------------
//  TEST: Anchor
//  
//  SCENARIO: 
//   1. Initialize the "anchor" contract (already done in "setup")
//   2. Check if the state/config matches the setup input
// ------------------------------------------------
async function testAnchorInitialize(
  junod: SigningCosmWasmClient,
  anchor: string,
): Promise<void> {
  process.stdout.write("Test - Anchor should initialize");
  const result: any = await junod.queryContractSmart(anchor, {
    config: {},
  });

  expect(result.handler == localjuno.contracts.anchorHandler);
  expect(result.proposal_nonce == 0);
  expect(result.chain_id == localjuno.contractsConsts.chainId);
  expect(result.tokenwrapper_addr == localjuno.contracts.tokenWrapper);
  expect(result.deposit_size == localjuno.contractsConsts.depositSize);

  console.log(chalk.green(" Passed!"));
}

// -----------------------------------------------
//  TEST: Anchor
//  
//  SCENARIO: 
//   1. Wallet3 "deposit" to anchor
//   2. Wallet2 "withdraw" from anchor
// ------------------------------------------------
async function testAnchorDepositWithdraw(
  junod: SigningCosmWasmClient,
  anchor: string,
  wallet1: DirectSecp256k1HdWallet,
  wallet2: DirectSecp256k1HdWallet,
  wallet3: DirectSecp256k1HdWallet,
  ucosm_amount: string,
): Promise<void> {
  process.stdout.write(`Test - Wallet3 deposit ${ucosm_amount} ucosm to anchor`);

  // Query the "amt_to_send" for "WrapAndDeposit" action
  const amt_to_send_query: any = await junod.queryContractSmart(localjuno.contracts.tokenWrapper, {
    get_amount_to_wrap: {
      target_amount: ucosm_amount,
    }
  });
  const ucosm_to_send = amt_to_send_query.amount_to_wrap;


  let wallet3_client = await SigningCosmWasmClient.connectWithSigner(
    localjuno.networkInfo.url,
    wallet3,
    {gasPrice: GasPrice.fromString("0.1ujunox")},
  );

  // Fail to "deposit" since no "commitment"
  await expect(
    wallet3_client.execute(localjuno.addresses.wallet3, localjuno.contracts.anchor, {
      wrap_and_deposit: {
        commitment: undefined,
        amount: ucosm_amount,
      }
    }, "auto", undefined, [coin(ucosm_to_send, "ucosm")])
  ).to.be.rejected; // rejectedWith("Commitment not found");

  // Succeed to "deposit"
  const result = await  wallet3_client.execute(localjuno.addresses.wallet3, localjuno.contracts.anchor, {
    wrap_and_deposit: {
      commitment: [114, 225, 36, 85, 19, 71, 228, 164, 174, 20, 198, 64, 177, 251, 100, 45, 249, 58, 6, 169, 158, 208, 56, 145, 80, 123, 65, 223, 143, 88, 145, 33],
      amount: ucosm_amount,
    }
  }, "auto", undefined, [coin(ucosm_to_send, "ucosm")])
  // console.log(result);

  process.stdout.write(`Test - Wallet2 withdraw ${ucosm_amount} WTW from anchor`);

  let wallet2_client = await SigningCosmWasmClient.connectWithSigner(
    localjuno.networkInfo.url,
    wallet2,
    {gasPrice: GasPrice.fromString("0.1ujunox")},
  );

  // Fail to "withdraw" since no "commitment"
  await expect(
    wallet2_client.execute(localjuno.addresses.wallet2, anchor, {
      withdraw_and_unwrap: {
        proof_bytes: [90, 249, 64, 247, 109, 43, 39, 43, 127, 147, 229, 67, 15, 213, 234, 24, 187, 126, 198, 37, 194, 70, 161, 33, 62, 18, 134, 53, 129, 165, 5, 10, 168, 232, 41, 122, 186, 111, 104, 142, 47, 66, 50, 172, 97, 255, 75, 254, 11, 254, 30, 154, 158, 24, 149, 136, 232, 227, 166, 90, 154, 212, 3, 39, 30, 20, 127, 166, 129, 102, 51, 233, 7, 46, 39, 179, 184, 10, 32, 148, 194, 253, 52, 33, 176, 125, 46, 157, 117, 52, 208, 18, 212, 0, 151, 136, 102, 212, 236, 123, 36, 167, 9, 133, 186, 37, 128, 123, 240, 179, 90, 33, 173, 96, 94, 98, 147, 11, 62, 131, 179, 3, 221, 162, 149, 147, 49, 160],
        roots: [[210, 149, 9, 63, 241, 232, 4, 209, 158, 207, 198, 252, 199, 227, 63, 215, 195, 25, 146, 122, 246, 212, 133, 210, 59, 166, 233, 91, 229, 28, 227, 23], [214, 149, 9, 63, 241, 232, 4, 209, 158, 207, 198, 252, 199, 227, 63, 215, 195, 25, 146, 122, 246, 212, 133, 210, 59, 166, 233, 91, 229, 28, 227, 23]],
        nullifier_hash: [20, 1, 74, 40, 205, 32, 60, 43, 111, 84, 9, 48, 56, 57, 117, 133, 54, 244, 112, 62, 103, 114, 20, 112, 43, 35, 144, 27, 227, 150, 56, 46],
        recipient: localjuno.addresses.wallet2,
        relayer: localjuno.addresses.wallet3,
        fee: "0", 
        refund: "0", 
        commitment: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        cw20_address: localjuno.contracts.tokenWrapper,
      },
    }, "auto", undefined, [])
  ).to.be.rejected; // rejectedWith("Root is not known");

  // Succeed to "withdraw" 
  const beforeBalance: Coin = await junod.getBalance(localjuno.addresses.wallet2, "ucosm");
  const beforeUcosm = beforeBalance.amount;

  const result1 = await wallet2_client.execute(localjuno.addresses.wallet2, anchor, {
    withdraw_and_unwrap: {
      proof_bytes: [90, 249, 64, 247, 109, 43, 39, 43, 127, 147, 229, 67, 15, 213, 234, 24, 187, 126, 198, 37, 194, 70, 161, 33, 62, 18, 134, 53, 129, 165, 5, 10, 168, 232, 41, 122, 186, 111, 104, 142, 47, 66, 50, 172, 97, 255, 75, 254, 11, 254, 30, 154, 158, 24, 149, 136, 232, 227, 166, 90, 154, 212, 3, 39, 30, 20, 127, 166, 129, 102, 51, 233, 7, 46, 39, 179, 184, 10, 32, 148, 194, 253, 52, 33, 176, 125, 46, 157, 117, 52, 208, 18, 212, 0, 151, 136, 102, 212, 236, 123, 36, 167, 9, 133, 186, 37, 128, 123, 240, 179, 90, 33, 173, 96, 94, 98, 147, 11, 62, 131, 179, 3, 221, 162, 149, 147, 49, 160],
      roots: [[214, 149, 9, 63, 241, 232, 4, 209, 158, 207, 198, 252, 199, 227, 63, 215, 195, 25, 146, 122, 246, 212, 133, 210, 59, 166, 233, 91, 229, 28, 227, 23], [214, 149, 9, 63, 241, 232, 4, 209, 158, 207, 198, 252, 199, 227, 63, 215, 195, 25, 146, 122, 246, 212, 133, 210, 59, 166, 233, 91, 229, 28, 227, 23]],
      nullifier_hash: [20, 1, 74, 40, 205, 32, 60, 43, 111, 84, 9, 48, 56, 57, 117, 133, 54, 244, 112, 62, 103, 114, 20, 112, 43, 35, 144, 27, 227, 150, 56, 46],
      recipient: localjuno.addresses.wallet2,
      relayer: localjuno.addresses.wallet3,
      fee: "0", 
      refund: "0", 
      commitment: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
      cw20_address: undefined,
    },
  }, "auto", undefined, []);

  const afterBalance: Coin = await junod.getBalance(localjuno.addresses.wallet2, "ucosm");
  const afterUcosm = afterBalance.amount;

  expect(parseInt(beforeUcosm) + parseInt(ucosm_amount) == parseInt(afterUcosm));

  console.log(chalk.green(" Passed!"));
}

// -----------------------------------------------
//  TEST: Anchor
//  
//  SCENARIO: 
//   1. Wallet3 "wrap"s some ucosm in anchor
// ------------------------------------------------
async function testAnchorWrapNative(
  junod: SigningCosmWasmClient,
  anchor: string,
  wallet3: DirectSecp256k1HdWallet,
  ucosm_amount: string,
): Promise<void> {
  process.stdout.write(`Test - Wallet3 wrap ${ucosm_amount} ucosm in anchor`);

  let wallet3_client = await SigningCosmWasmClient.connectWithSigner(
    localjuno.networkInfo.url, 
    wallet3, 
    { gasPrice: GasPrice.fromString("0.1ujunox") },
  );

  const beforeBalance: any = await junod.queryContractSmart(localjuno.contracts.tokenWrapper, {
    balance: {
      address: localjuno.addresses.wallet3,
    }
  });
  const beforeWTW = beforeBalance.balance;

  const result = await wallet3_client.execute(localjuno.addresses.wallet3, anchor, {
      wrap_native: {
        amount: ucosm_amount, 
      },
    },
    "auto", undefined, [coin(ucosm_amount, "ucosm")]
  );

  
  const afterBalance: any = await junod.queryContractSmart(localjuno.contracts.tokenWrapper, {
    balance: {
      address: localjuno.addresses.wallet3,
    }
  });
  const afterWTW = afterBalance.balance;

  // Here, we knows that the "fee_percentage" is "0.1".
  expect(parseInt(beforeWTW) + parseInt(ucosm_amount) * 0.9 == parseInt(afterWTW));

  console.log(chalk.green(" Passed!"));
}

// -----------------------------------------------
//  TEST: Anchor
//  
//  SCENARIO: 
//   1. Wallet3 "unwrap"s some WTW in anchor
// ------------------------------------------------
async function testAnchorUnwrapNative(
  junod: SigningCosmWasmClient,
  anchor: string,
  wallet3: DirectSecp256k1HdWallet,
  wtw_amount: string,
): Promise<void> {
  process.stdout.write(`Test - Wallet3 unwrap ${wtw_amount} WTW in anchor`);

  let wallet3_client = await SigningCosmWasmClient.connectWithSigner(
    localjuno.networkInfo.url, 
    wallet3, 
    { gasPrice: GasPrice.fromString("0.1ujunox") },
  );

  const beforeBalance: Coin = await junod.getBalance(localjuno.addresses.wallet3, "ucosm");
  const beforeUcosm = beforeBalance.amount;

  const result = await wallet3_client.execute(localjuno.addresses.wallet3, anchor, {
      unwrap_native: {
        amount: wtw_amount, 
      },
    },
    "auto", undefined, []
  );
  
  const afterBalance: Coin = await junod.getBalance(localjuno.addresses.wallet3, "ucosm");
  const afterUcosm = afterBalance.amount;

  expect(parseInt(beforeUcosm) + parseInt(wtw_amount) == parseInt(afterUcosm));

  console.log(chalk.green(" Passed!"));
}