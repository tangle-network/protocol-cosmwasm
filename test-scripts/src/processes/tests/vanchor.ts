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
//  TEST: VAnchor
//  
//  SCENARIO: 
//   1. Initialize the "vanchor" contract (already done in "setup")
//   2. Check if the state/config matches the setup input
// ------------------------------------------------
export async function testVAnchorInitialize(
    junod: SigningCosmWasmClient,
    vanchor: string,
  ): Promise<void> {
    process.stdout.write("Test - VAnchor should initialize");
    const result: any = await junod.queryContractSmart(vanchor, {
      config: {},
    });
  
    expect(result.handler == localjuno.contracts.anchorHandler);
    expect(result.proposal_nonce == 0);
    expect(result.chain_id == localjuno.contractsConsts.chainId);
    expect(result.tokenwrapper_addr == localjuno.contracts.tokenWrapper);
    expect(result.max_deposit_amt == localjuno.contractsConsts.maxDepositAmt);
    expect(result.min_withdraw_amt == localjuno.contractsConsts.minWithdrawAmt);
    expect(result.max_ext_amt == localjuno.contractsConsts.maxExtAmt);
    expect(result.max_fee == localjuno.contractsConsts.maxFee);
  
    console.log(chalk.green(" Passed!"));
  }
  
  // -----------------------------------------------
  //  TEST: VAnchor
  //  
  //  SCENARIO: 
  //   1. Wallet3 "deposit" to anchor
  //   2. Wallet2 "withdraw" from anchor
  // ------------------------------------------------
  export async function testVAnchorDepositWithdraw(
    junod: SigningCosmWasmClient,
    vanchor: string,
    auto: string,
    wallet1: DirectSecp256k1HdWallet,
    wallet2: DirectSecp256k1HdWallet,
    wallet3: DirectSecp256k1HdWallet,
    in_auto_public_amount: string,
    in_ext_amount: string,
    in_fee: string,
    out_ucosm_public_amount: string,
    out_ext_amount: string,
    out_fee: string,
  ): Promise<void> {
    process.stdout.write(`Test - Wallet3 deposit ${in_auto_public_amount} AUTO to vanchor`); 

    // Query the "amt_to_send" for "WrapAndDeposit" action
    const amt_to_send_query: any = await junod.queryContractSmart(localjuno.contracts.tokenWrapper, {
      get_amount_to_wrap: {
        target_amount: in_auto_public_amount,
      }
    });
    const auto_to_send = amt_to_send_query.amount_to_wrap;

    let wallet3_client = await SigningCosmWasmClient.connectWithSigner(
      localjuno.networkInfo.url,
      wallet3,
      {gasPrice: GasPrice.fromString("0.1ujunox")},
    );

    // Before any deposit, "wallet3" increases the allowance
    await wallet3_client.execute(localjuno.addresses.wallet3, auto, {
      increase_allowance: {
        spender: vanchor,
        amount: "1000000000",
        expires: undefined,
      }
    }, "auto", undefined, []);
  
    // Fail to "deposit" since no "proof_data" & "ext_data"
    let transactDepositWrapMsg = toEncodedBinary({
      transact_deposit_wrap: {
        proof_data: undefined,
        ext_data: undefined,
      }
    })
    await expect(
      wallet3_client.execute(localjuno.addresses.wallet3, auto, {
        send: {
          contract: vanchor,
          amount: auto_to_send, 
          msg: transactDepositWrapMsg,
        },
      }, "auto", undefined, [])
    ).to.be.rejected; // rejectedWith("Commitment not found");
  
    // Succeed to "deposit"
    let depositProofData = { 
      proof: [249, 63, 95, 77, 91, 215, 254, 58, 26, 170, 201, 237, 125, 157, 247, 143, 206, 218, 154, 56, 205, 12, 100, 134, 113, 235, 149, 53, 212, 99, 217, 142, 174, 68, 235, 82, 51, 193, 43, 178, 26, 128, 44, 121, 29, 218, 106, 107, 88, 143, 93, 26, 75, 200, 168, 173, 51, 169, 36, 67, 255, 105, 234, 25, 205, 146, 192, 16, 145, 11, 47, 4, 254, 134, 151, 174, 6, 178, 110, 86, 59, 190, 67, 5, 122, 177, 1, 189, 149, 238, 234, 135, 118, 239, 176, 149, 164, 68, 75, 163, 237, 7, 210, 183, 249, 185, 86, 82, 163, 54, 193, 236, 28, 196, 17, 23, 76, 142, 54, 154, 86, 36, 220, 121, 39, 11, 19, 31], 
      public_amount: [10, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0], 
      roots: [[239, 89, 25, 14, 35, 42, 26, 61, 180, 140, 224, 106, 63, 122, 122, 78, 89, 65, 28, 26, 74, 63, 65, 52, 176, 152, 45, 245, 107, 212, 24, 9], [239, 89, 25, 14, 35, 42, 26, 61, 180, 140, 224, 106, 63, 122, 122, 78, 89, 65, 28, 26, 74, 63, 65, 52, 176, 152, 45, 245, 107, 212, 24, 9]], 
      input_nullifiers: [[83, 180, 139, 65, 56, 112, 24, 191, 212, 50, 135, 164, 184, 126, 193, 240, 161, 130, 245, 6, 63, 114, 230, 13, 221, 253, 248, 164, 249, 208, 50, 17], [211, 3, 245, 77, 123, 200, 59, 116, 120, 218, 244, 44, 163, 180, 233, 15, 17, 205, 20, 134, 149, 32, 203, 230, 174, 5, 41, 45, 2, 90, 51, 13]], 
      output_commitments: [[20, 75, 155, 185, 13, 156, 191, 68, 189, 39, 70, 163, 41, 46, 77, 108, 110, 192, 77, 226, 44, 203, 243, 44, 254, 89, 14, 136, 236, 79, 58, 29], [196, 160, 156, 49, 22, 132, 172, 188, 3, 250, 188, 30, 145, 3, 235, 15, 31, 179, 214, 170, 238, 161, 154, 42, 170, 39, 123, 249, 145, 138, 181, 29]], 
      ext_data_hash: [222, 216, 114, 136, 124, 143, 207, 214, 121, 161, 68, 249, 147, 71, 109, 45, 125, 223, 141, 196, 35, 109, 148, 196, 173, 32, 126, 248, 1, 226, 81, 39] 
    };
    let depositExtData = { 
      recipient: localjuno.addresses.wallet2, 
      relayer: localjuno.addresses.wallet1, 
      ext_amount: in_ext_amount, 
      fee: in_fee, 
      encrypted_output1: [20, 75, 155, 185, 13, 156, 191, 68, 189, 39, 70, 163, 41, 46, 77, 108, 110, 192, 77, 226, 44, 203, 243, 44, 254, 89, 14, 136, 236, 79, 58, 29], 
      encrypted_output2: [196, 160, 156, 49, 22, 132, 172, 188, 3, 250, 188, 30, 145, 3, 235, 15, 31, 179, 214, 170, 238, 161, 154, 42, 170, 39, 123, 249, 145, 138, 181, 29],
    };

    transactDepositWrapMsg = toEncodedBinary({
      transact_deposit_wrap: {
        proof_data: depositProofData,
        ext_data: depositExtData,
      }
    });
    const _result = await wallet3_client.execute(
      localjuno.addresses.wallet3, 
      auto, 
      {
        send: {
          contract: vanchor,
          amount: auto_to_send, 
          msg: transactDepositWrapMsg,
        },
      }, 
      "auto", undefined, []);
      
    console.log(chalk.green(" Passed!\n"));
  
    process.stdout.write(`Test - Wallet2 withdraw ${in_auto_public_amount} WTW from vanchor`);
  
    let wallet2_client = await SigningCosmWasmClient.connectWithSigner(
      localjuno.networkInfo.url,
      wallet2,
      {gasPrice: GasPrice.fromString("0.1ujunox")},
    );
  
    // Fail to "withdraw" since no "commitment"
    await expect(
      wallet2_client.execute(localjuno.addresses.wallet2, vanchor, {
        transact_withdraw_unwrap: {
          proof_data: undefined,
          ext_data: undefined,
          token_addr: undefined,
        },
      }, "auto", undefined, [])
    ).to.be.rejected; // rejectedWith("Root is not known");
  
    // Succeed to "withdraw" 
    const beforeBalance: Coin = await junod.getBalance(localjuno.addresses.wallet2, "ucosm");
    const beforeUcosm = beforeBalance.amount;
      
    let withdrawProofData = { 
      proof: [105, 7, 193, 151, 128, 163, 45, 175, 77, 115, 231, 108, 226, 228, 26, 193, 84, 92, 140, 202, 66, 187, 119, 209, 102, 35, 211, 66, 7, 153, 96, 20, 157, 71, 81, 253, 212, 37, 121, 176, 61, 226, 63, 24, 102, 224, 147, 94, 193, 171, 174, 249, 174, 113, 216, 248, 242, 118, 90, 18, 101, 229, 86, 15, 176, 58, 173, 137, 165, 35, 24, 71, 158, 128, 173, 103, 139, 45, 131, 247, 178, 115, 251, 148, 224, 223, 152, 4, 65, 49, 25, 41, 252, 78, 154, 22, 26, 142, 180, 44, 175, 144, 72, 243, 241, 179, 141, 190, 70, 49, 100, 67, 222, 207, 188, 189, 163, 198, 26, 222, 164, 47, 122, 144, 143, 210, 234, 162], 
      public_amount: [250, 255, 255, 239, 147, 245, 225, 67, 145, 112, 185, 121, 72, 232, 51, 40, 93, 88, 129, 129, 182, 69, 80, 184, 41, 160, 49, 225, 114, 78, 100, 48], 
      roots: [[45, 21, 249, 142, 150, 215, 102, 179, 25, 192, 13, 7, 191, 101, 47, 62, 233, 117, 143, 207, 70, 159, 128, 227, 80, 80, 162, 50, 255, 166, 110, 41], [45, 21, 249, 142, 150, 215, 102, 179, 25, 192, 13, 7, 191, 101, 47, 62, 233, 117, 143, 207, 70, 159, 128, 227, 80, 80, 162, 50, 255, 166, 110, 41]], 
      input_nullifiers: [[0, 83, 230, 137, 230, 209, 253, 209, 28, 80, 92, 11, 41, 47, 175, 127, 209, 225, 57, 230, 1, 11, 4, 66, 92, 77, 120, 38, 129, 185, 66, 18], [232, 233, 88, 214, 173, 131, 231, 212, 227, 33, 2, 22, 236, 192, 180, 218, 27, 151, 77, 243, 134, 144, 86, 115, 1, 182, 148, 34, 17, 188, 106, 7]], 
      output_commitments: [[236, 125, 91, 108, 52, 47, 35, 195, 65, 197, 8, 183, 44, 160, 29, 149, 250, 218, 83, 10, 14, 80, 138, 83, 41, 72, 216, 79, 202, 91, 189, 31], [3, 48, 30, 43, 113, 185, 39, 226, 100, 156, 46, 26, 190, 67, 190, 116, 123, 17, 122, 116, 72, 225, 43, 137, 210, 142, 67, 211, 160, 4, 179, 15]],
      ext_data_hash: [158, 9, 138, 113, 229, 26, 141, 207, 13, 240, 39, 98, 110, 41, 246, 100, 171, 99, 51, 92, 170, 131, 208, 76, 119, 5, 32, 149, 87, 129, 60, 11] 
    };
    let withdrawExtData = { 
      recipient: localjuno.addresses.wallet2, 
      relayer: localjuno.addresses.wallet1, 
      ext_amount: out_ext_amount, 
      fee: out_fee, 
      encrypted_output1: [236, 125, 91, 108, 52, 47, 35, 195, 65, 197, 8, 183, 44, 160, 29, 149, 250, 218, 83, 10, 14, 80, 138, 83, 41, 72, 216, 79, 202, 91, 189, 31], 
      encrypted_output2: [3, 48, 30, 43, 113, 185, 39, 226, 100, 156, 46, 26, 190, 67, 190, 116, 123, 17, 122, 116, 72, 225, 43, 137, 210, 142, 67, 211, 160, 4, 179, 15]
    };
    const result1 = await wallet2_client.execute(localjuno.addresses.wallet2, vanchor, {
      transact_withdraw_unwrap: {
        proof_data: withdrawProofData,
        ext_data: withdrawExtData,
        token_addr: undefined,
      },
    }, "auto", undefined, []);
  
    const afterBalance: Coin = await junod.getBalance(localjuno.addresses.wallet2, "ucosm");
    const afterUcosm = afterBalance.amount;
  
    expect(parseInt(beforeUcosm) + Math.abs(parseInt(out_ucosm_public_amount)) == parseInt(afterUcosm));
  
    console.log(chalk.green(" Passed!"));
  }
  
  // -----------------------------------------------
  //  TEST: VAnchor
  //  
  //  SCENARIO: 
  //   1. Wallet3 "wrap"s some ucosm in anchor
  // ------------------------------------------------
  export async function testVAnchorWrapNative(
    junod: SigningCosmWasmClient,
    vanchor: string,
    wallet3: DirectSecp256k1HdWallet,
    ucosm_amount: string,
  ): Promise<void> {
    process.stdout.write(`Test - Wallet3 wrap ${ucosm_amount} ucosm in vanchor`);
  
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
  
    const result = await wallet3_client.execute(localjuno.addresses.wallet3, vanchor, {
        wrap_native: {
          amount: ucosm_amount, 
          is_deposit: false,
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
  //  TEST: VAnchor
  //  
  //  SCENARIO: 
  //   1. Wallet3 "unwrap"s some WTW in anchor
  // ------------------------------------------------
  export async function testVAnchorUnwrapNative(
    junod: SigningCosmWasmClient,
    vanchor: string,
    wallet3: DirectSecp256k1HdWallet,
    wtw_amount: string,
  ): Promise<void> {
    process.stdout.write(`Test - Wallet3 unwrap ${wtw_amount} WTW in vanchor`);
  
    let wallet3_client = await SigningCosmWasmClient.connectWithSigner(
      localjuno.networkInfo.url, 
      wallet3, 
      { gasPrice: GasPrice.fromString("0.1ujunox") },
    );
  
    const beforeBalance: Coin = await junod.getBalance(localjuno.addresses.wallet3, "ucosm");
    const beforeUcosm = beforeBalance.amount;
  
    const result = await wallet3_client.execute(localjuno.addresses.wallet3, vanchor, {
        unwrap_native: {
          amount: wtw_amount, 
          recipient: localjuno.addresses.wallet3,
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
  //  TEST: VAnchor
  //  
  //  SCENARIO: 
  //   1. Wallet3 "wrap"s some CW20 token(AUTO) in anchor
  // ------------------------------------------------
  export async function testVAnchorWrapCw20(
    junod: SigningCosmWasmClient,
    vanchor: string,
    tokenWrapper: string,
    auto: string,
    wallet3: DirectSecp256k1HdWallet,
    auto_amount: string,
  ): Promise<void> {
    process.stdout.write(`Test - Wallet3 wrap ${auto_amount} AUTO in vanchor`);
  
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
      wrap_token: {
        is_deposit: false,
      },
    });
  
    const result = await wallet3_client.execute(localjuno.addresses.wallet3, auto, {
        send: {
          contract: vanchor,
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
  //  TEST: VAnchor
  //  
  //  SCENARIO: 
  //   1. Wallet3 "unwrap"s some WTW to Cw20 token(AUTO) in vanchor
  // ------------------------------------------------
  export async function testVAnchorUnwrapCw20(
    junod: SigningCosmWasmClient,
    vanchor: string,
    tokenWrapper: string,
    auto: string,
    wallet3: DirectSecp256k1HdWallet,
    wtw_amount: string,
  ): Promise<void> {
    process.stdout.write(`Test - Wallet3 unwrap ${wtw_amount} WTW in vanchor`);
  
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
  
    const result = await wallet3_client.execute(localjuno.addresses.wallet3, vanchor, {
        unwrap_into_token: {
          token_addr: auto,
          amount: wtw_amount, 
          recipient: localjuno.addresses.wallet3,
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