import { SigningCosmWasmClient } from "@cosmjs/cosmwasm-stargate";
import { coin, DirectSecp256k1HdWallet } from "@cosmjs/proto-signing";
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

  // Mixer
  
  process.exit();
}


// -----------------------------------------------
//  TEST: "FundsRouter" fails to forward the call
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
//  TEST: "FundsRouter" fails to forward the call
//  
//  SCENARIO: 
//   1. Invalid caller(non-"reg_user_fee_forwarder") sends the "FcnData".
//   2. Valid caller("reg_user_fee_forwarder" wallet2) sends insufficient funds.
// ------------------------------------------------
async function testFundsRouterForwardCallsFails(
  junod: SigningCosmWasmClient,
  wallet1: DirectSecp256k1HdWallet, 
  wallet2: DirectSecp256k1HdWallet,
  wallet3: DirectSecp256k1HdWallet,
  fundsRouter: string,
  fee_amount: string,
  ucosm_amount: string,
): Promise<void> {
  process.stdout.write("Test - FundsRouter ForwardCalls fails");

  let wallet3_client = await SigningCosmWasmClient.connectWithSigner(
    localjuno.networkInfo.url, 
    wallet3, 
    {gasPrice: GasPrice.fromString("0.1ujunox")},
  );

  await expect(
    wallet3_client.execute(localjuno.addresses.wallet3, fundsRouter, {
      forward_calls: {
        user: localjuno.addresses.wallet1,
        fee_amount: fee_amount,
        fcn_data: [],
      },
    }, "auto", undefined, [coin(ucosm_amount, "ucosm")])
  ).to.be.rejected; // rejectedWith("FRouter: not userFeeForw");
  
  const wallet2_client = await SigningCosmWasmClient.connectWithSigner(
    localjuno.networkInfo.url,
    wallet2, 
    {gasPrice: GasPrice.fromString("0.1ujunox")},
  );
  await expect(
    wallet2_client.execute(localjuno.addresses.wallet2, fundsRouter, {
      forward_calls: {
        user: localjuno.addresses.wallet1,
        fee_amount: fee_amount,
        fcn_data: [],
      },
    }, "auto", undefined, [coin(ucosm_amount, "ucosm")])
  ).to.be.rejected;  // rejectedWith("Insufficent funds");
 
  console.log(chalk.green(" Passed!"));
}

// -----------------------------------------------
//  TEST: "FundsRouter" successfully forwards the call
//  
//  SCENARIO: 
//   "reg_user_fee_forwarder"(wallet2) "increment"s the "counter" state
//   by sending the "FcnData"("increment")
// ------------------------------------------------
async function testFundsRouterForwardCallsSuccess(
  junod: SigningCosmWasmClient,
  wallet1: DirectSecp256k1HdWallet, 
  wallet2: DirectSecp256k1HdWallet,
  wallet3: DirectSecp256k1HdWallet,
  fundsRouter: string,
  fcnData: any,
  fee_amount: string,
  ucosm_amount: string,
  counter: string,
): Promise<void> {
  process.stdout.write("Test - FundsRouter ForwardCalls Success");
 
  const beforeCountQuery: any = await junod.queryContractSmart(counter, {
    get_count: {},
  });

 let wallet2_client = await SigningCosmWasmClient.connectWithSigner(
   localjuno.networkInfo.url, 
   wallet2, 
   {gasPrice: GasPrice.fromString("0.1ujunox")},
  );
  const result = await wallet2_client.execute(
    localjuno.addresses.wallet2, 
    fundsRouter, 
    {
      forward_calls: {
        user: localjuno.addresses.wallet1,
        fee_amount: fee_amount,
        fcn_data: fcnData,
      },
    }, 
    "auto", 
    undefined, 
    [coin(ucosm_amount, "ucosm")],
  );
  expect(result).to.be.ok;

  // Check the "increment" result.
  // process.stdout.write("\nQuery the counter contract");
  const afterCountQuery: any = await junod.queryContractSmart(counter, {
    get_count: {},
  })
  // console.log(afterCountQuery);
  expect(afterCountQuery.count == (beforeCountQuery.count + 1)).to.be.ok;

  console.log(chalk.green(" Passed!"));
}

// -----------------------------------------------
//  TEST: Wallet3 successfully "deposit" "ucosm" token to "FundsRouter"
//  
//  SCENARIO: 
//    Wallet3 deposits 100 "ucosm" for himself
// ------------------------------------------------
async function testFundsRouterDepositNativeSuccess(
  junod: SigningCosmWasmClient,
  wallet3: DirectSecp256k1HdWallet,
  fundsRouter: string,
  ucosm_amount: string,
  spender: string,
): Promise<void> {
  process.stdout.write("Test - FundsRouter DepositNative Success");
 
  const beforeBalanceQuery: any = await junod.queryContractSmart(fundsRouter, {
    get_balance: { user_addr: spender },
  });

 let wallet3_client = await SigningCosmWasmClient.connectWithSigner(
   localjuno.networkInfo.url, 
   wallet3, 
   {gasPrice: GasPrice.fromString("0.1ujunox")},
  );
  const result = await wallet3_client.execute(
    localjuno.addresses.wallet3, 
    fundsRouter, 
    {
      deposit_native: { spender: spender },
    }, 
    "auto", 
    undefined, 
    [coin(ucosm_amount, "ucosm")],
  );
  expect(result).to.be.ok;

  // Check the "deposit_native" result.
  // process.stdout.write(`\nQuery the Balance of ${spender}`);
  const afterBalanceQuery: any = await junod.queryContractSmart(fundsRouter, {
    get_balance: { user_addr: spender },
  })
  // console.log(afterBalanceQuery);
  expect(
    parseInt(afterBalanceQuery.balance) 
    == parseInt(beforeBalanceQuery.balance) + parseInt(ucosm_amount)
  ).to.be.ok;

  console.log(chalk.green(" Passed!"));
}


// -----------------------------------------------
//  TEST: Wallet3 successfully "deposit" "ucosm" token to "FundsRouter"
//  
//  SCENARIO: 
//    Wallet3 withdraws 100 "ucosm" for himself
// ------------------------------------------------
async function testFundsRouterWithdrawNativeSuccess(
  junod: SigningCosmWasmClient,
  wallet3: DirectSecp256k1HdWallet,
  fundsRouter: string,
  ucosm_amount: string,
  recipient: string,
): Promise<void> {
  process.stdout.write("Test - FundsRouter WithdrawNative Success");
 
  const beforeBalanceQuery: any = await junod.queryContractSmart(fundsRouter, {
    get_balance: { user_addr: recipient },
  });

 let wallet3_client = await SigningCosmWasmClient.connectWithSigner(
   localjuno.networkInfo.url, 
   wallet3, 
   {gasPrice: GasPrice.fromString("0.1ujunox")},
  );
  const result = await wallet3_client.execute(
    localjuno.addresses.wallet3, 
    fundsRouter, 
    {
      withdraw_native: { recipient: recipient, amount: ucosm_amount },
    }, 
    "auto", 
    undefined, 
    [],
  );
  expect(result).to.be.ok;

  // Check the "deposit_native" result.
  // process.stdout.write(`\nQuery the Balance of ${recipient}`);
  const afterBalanceQuery: any = await junod.queryContractSmart(fundsRouter, {
    get_balance: { user_addr: recipient },
  })
  // console.log(afterBalanceQuery);
  expect(
    parseInt(afterBalanceQuery.balance) 
    == parseInt(beforeBalanceQuery.balance) - parseInt(ucosm_amount)
  ).to.be.ok;

  console.log(chalk.green(" Passed!"));
}



// -----------------------
//  Querying tests
// -----------------------
async function testQueryFundsRouterState(
  junod: SigningCosmWasmClient,
  fundsRouter: string,
): Promise<void> {
  process.stdout.write("Test - Query FundsRouter state");
  const result: any = await junod.queryContractSmart(fundsRouter, {
    get_state: {},
  });

  console.log(result);
  console.log(chalk.green(" Passed!"));
}