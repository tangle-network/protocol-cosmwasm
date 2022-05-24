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

// -----------------------------------------------
//  TEST: Anchor
//  
//  SCENARIO: 
//   1. Initialize the "anchor" contract (already done in "setup")
//   2. Check if the state/config matches the setup input
// ------------------------------------------------
export async function testAnchorInitialize(
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
  export async function testAnchorDepositWithdraw(
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
  export async function testAnchorWrapNative(
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
  export async function testAnchorUnwrapNative(
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
  
  // -----------------------------------------------
  //  TEST: Anchor
  //  
  //  SCENARIO: 
  //   1. Wallet3 "wrap"s some CW20 token(AUTO) in anchor
  // ------------------------------------------------
  export async function testAnchorWrapCw20(
    junod: SigningCosmWasmClient,
    anchor: string,
    tokenWrapper: string,
    auto: string,
    wallet3: DirectSecp256k1HdWallet,
    auto_amount: string,
  ): Promise<void> {
    process.stdout.write(`Test - Wallet3 wrap ${auto_amount} AUTO in anchor`);
  
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
  
    const wrapCw20Msg = toEncodedBinary({
      wrap_token: {},
    });
  
    const result = await wallet3_client.execute(localjuno.addresses.wallet3, auto, {
        send: {
          contract: anchor,
          amount: auto_amount, 
          msg: wrapCw20Msg,
        },
      },
      "auto", undefined, []
    );
  
    
    const afterBalance: any = await junod.queryContractSmart(localjuno.contracts.tokenWrapper, {
      balance: {
        address: localjuno.addresses.wallet3,
      }
    });
    const afterWTW = afterBalance.balance;
  
    // Here, we knows that the "fee_percentage" is "0.1".
    expect(parseInt(beforeWTW) + parseInt(auto_amount) * 0.9 == parseInt(afterWTW));
  
    console.log(chalk.green(" Passed!"));
  }
  
  // -----------------------------------------------
  //  TEST: Anchor
  //  
  //  SCENARIO: 
  //   1. Wallet3 "unwrap"s some WTW to Cw20 token(AUTO) in anchor
  // ------------------------------------------------
  export async function testAnchorUnwrapCw20(
    junod: SigningCosmWasmClient,
    anchor: string,
    tokenWrapper: string,
    auto: string,
    wallet3: DirectSecp256k1HdWallet,
    wtw_amount: string,
  ): Promise<void> {
    process.stdout.write(`Test - Wallet3 unwrap ${wtw_amount} WTW in anchor`);
  
    let wallet3_client = await SigningCosmWasmClient.connectWithSigner(
      localjuno.networkInfo.url, 
      wallet3, 
      { gasPrice: GasPrice.fromString("0.1ujunox") },
    );
  
    const beforeBalance: any = await junod.queryContractSmart(auto, {
      balance: {
        address: localjuno.addresses.wallet3 
      }
    });
    const beforeAUTO = beforeBalance.balance;
  
    const result = await wallet3_client.execute(localjuno.addresses.wallet3, anchor, {
        unwrap_into_token: {
          token_addr: auto,
          amount: wtw_amount, 
        },
      },
      "auto", undefined, []
    );
    
    const afterBalance: any =  await junod.queryContractSmart(auto, {
      balance: {
        address: localjuno.addresses.wallet3 
      }
    });
    const afterAUTO = beforeBalance.balance;
    expect(parseInt(beforeAUTO) + parseInt(wtw_amount) == parseInt(afterAUTO));
  
    console.log(chalk.green(" Passed!"));
  }