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
        commitment: [168, 247, 178, 1, 255, 94, 65, 44, 230, 251, 95, 202, 71, 133, 55, 218, 109, 250, 240, 191, 208, 58, 227, 252, 77, 90, 234, 98, 145, 24, 141, 43],
        amount: ucosm_amount,
      }
    }, "auto", undefined, [coin(ucosm_to_send, "ucosm")])
      
    console.log(chalk.green(" Passed!\n"));
  
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
          proof_bytes: [102, 172, 207, 127, 45, 32, 170, 13, 146, 22, 201, 77, 129, 236, 140, 134, 138, 157, 89, 187, 57, 214, 228, 100, 48, 2, 156, 130, 157, 81, 29, 130, 8, 229, 108, 16, 176, 143, 130, 96, 150, 170, 252, 72, 33, 69, 174, 214, 34, 50, 253, 22, 30, 30, 12, 202, 243, 199, 173, 84, 137, 110, 230, 2, 174, 119, 59, 165, 185, 179, 54, 221, 134, 194, 126, 110, 176, 236, 32, 141, 5, 152, 64, 245, 16, 185, 22, 253, 1, 136, 167, 215, 66, 205, 152, 37, 208, 104, 45, 88, 112, 35, 38, 9, 67, 106, 51, 12, 15, 163, 105, 64, 169, 57, 196, 32, 99, 7, 60, 40, 207, 11, 183, 205, 27, 97, 158, 129],
          roots: [],
          nullifier_hash: [183, 160, 141, 89, 98, 241, 220, 87, 120, 249, 242, 56, 92, 41, 28, 230, 247, 111, 155, 7, 94, 2, 142, 101, 0, 243, 39, 32, 59, 235, 198, 31],
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
        proof_bytes: [102, 172, 207, 127, 45, 32, 170, 13, 146, 22, 201, 77, 129, 236, 140, 134, 138, 157, 89, 187, 57, 214, 228, 100, 48, 2, 156, 130, 157, 81, 29, 130, 8, 229, 108, 16, 176, 143, 130, 96, 150, 170, 252, 72, 33, 69, 174, 214, 34, 50, 253, 22, 30, 30, 12, 202, 243, 199, 173, 84, 137, 110, 230, 2, 174, 119, 59, 165, 185, 179, 54, 221, 134, 194, 126, 110, 176, 236, 32, 141, 5, 152, 64, 245, 16, 185, 22, 253, 1, 136, 167, 215, 66, 205, 152, 37, 208, 104, 45, 88, 112, 35, 38, 9, 67, 106, 51, 12, 15, 163, 105, 64, 169, 57, 196, 32, 99, 7, 60, 40, 207, 11, 183, 205, 27, 97, 158, 129],
        roots: [[193, 227, 188, 65, 69, 113, 210, 2, 150, 35, 62, 52, 41, 169, 144, 213, 69, 59, 17, 14, 106, 181, 222, 51, 208, 134, 119, 184, 130, 215, 145, 46], [193, 227, 188, 65, 69, 113, 210, 2, 150, 35, 62, 52, 41, 169, 144, 213, 69, 59, 17, 14, 106, 181, 222, 51, 208, 134, 119, 184, 130, 215, 145, 46]],
        nullifier_hash: [183, 160, 141, 89, 98, 241, 220, 87, 120, 249, 242, 56, 92, 41, 28, 230, 247, 111, 155, 7, 94, 2, 142, 101, 0, 243, 39, 32, 59, 235, 198, 31],
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