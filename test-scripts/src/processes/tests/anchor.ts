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
        commitment: [235, 3, 87, 22, 66, 107, 162, 89, 147, 182, 207, 54, 195, 138, 136, 110, 150, 128, 213, 43, 44, 121, 177, 9, 17, 54, 9, 202, 11, 230, 158, 8],
        amount: ucosm_amount,
      }
    }, "auto", undefined, [coin(ucosm_to_send, "ucosm")])
    // console.log(result);
      
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
          proof_bytes: [168, 60, 122, 239, 216, 49, 51, 19, 135, 224, 117, 57, 71, 178, 12, 149, 135, 132, 115, 166, 248, 175, 71, 160, 160, 92, 167, 35, 250, 197, 156, 13, 82, 106, 210, 191, 192, 174, 127, 23, 146, 114, 67, 179, 210, 230, 89, 73, 81, 109, 204, 170, 1, 80, 193, 229, 101, 58, 38, 204, 90, 111, 45, 20, 106, 206, 173, 139, 4, 104, 18, 116, 216, 41, 10, 147, 221, 99, 98, 42, 229, 213, 30, 251, 131, 33, 236, 31, 75, 76, 29, 155, 85, 244, 59, 162, 173, 48, 143, 49, 233, 38, 135, 62, 5, 125, 99, 61, 72, 168, 208, 187, 98, 141, 85, 129, 247, 145, 102, 19, 205, 100, 75, 200, 20, 205, 92, 168],
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
        proof_bytes: [168, 60, 122, 239, 216, 49, 51, 19, 135, 224, 117, 57, 71, 178, 12, 149, 135, 132, 115, 166, 248, 175, 71, 160, 160, 92, 167, 35, 250, 197, 156, 13, 82, 106, 210, 191, 192, 174, 127, 23, 146, 114, 67, 179, 210, 230, 89, 73, 81, 109, 204, 170, 1, 80, 193, 229, 101, 58, 38, 204, 90, 111, 45, 20, 106, 206, 173, 139, 4, 104, 18, 116, 216, 41, 10, 147, 221, 99, 98, 42, 229, 213, 30, 251, 131, 33, 236, 31, 75, 76, 29, 155, 85, 244, 59, 162, 173, 48, 143, 49, 233, 38, 135, 62, 5, 125, 99, 61, 72, 168, 208, 187, 98, 141, 85, 129, 247, 145, 102, 19, 205, 100, 75, 200, 20, 205, 92, 168],
        roots: [[95, 118, 152, 109, 95, 8, 90, 146, 31, 73, 193, 217, 18, 210, 109, 187, 210, 188, 213, 120, 236, 144, 71, 85, 80, 77, 197, 35, 135, 170, 135, 45], [95, 118, 152, 109, 95, 8, 90, 146, 31, 73, 193, 217, 18, 210, 109, 187, 210, 188, 213, 120, 236, 144, 71, 85, 80, 77, 197, 35, 135, 170, 135, 45]],
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