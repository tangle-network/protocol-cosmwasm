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
    process.stdout.write(`Test - Wallet3 deposit ${in_auto_public_amount} ABCT to vanchor`); 

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
      proof: [246, 224, 80, 136, 59, 132, 218, 229, 172, 74, 200, 10, 206, 194, 180, 162, 56, 36, 224, 54, 164, 214, 121, 189, 91, 130, 24, 163, 98, 145, 60, 33, 205, 91, 41, 33, 88, 173, 67, 242, 24, 40, 127, 27, 227, 179, 184, 63, 6, 198, 189, 46, 117, 48, 112, 214, 13, 192, 192, 118, 134, 141, 76, 29, 99, 72, 157, 10, 59, 228, 47, 1, 174, 91, 62, 95, 42, 155, 72, 141, 245, 51, 83, 37, 219, 156, 151, 224, 215, 147, 162, 28, 177, 23, 109, 144, 1, 148, 94, 145, 189, 66, 186, 191, 234, 193, 41, 151, 219, 15, 188, 31, 98, 70, 196, 8, 176, 216, 23, 192, 191, 212, 96, 152, 236, 111, 182, 24], 
      public_amount: [10, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0], 
      roots: [[239, 89, 25, 14, 35, 42, 26, 61, 180, 140, 224, 106, 63, 122, 122, 78, 89, 65, 28, 26, 74, 63, 65, 52, 176, 152, 45, 245, 107, 212, 24, 9], [239, 89, 25, 14, 35, 42, 26, 61, 180, 140, 224, 106, 63, 122, 122, 78, 89, 65, 28, 26, 74, 63, 65, 52, 176, 152, 45, 245, 107, 212, 24, 9]], 
      input_nullifiers: [[73, 41, 139, 22, 139, 211, 80, 235, 119, 230, 223, 95, 126, 27, 81, 248, 244, 81, 77, 14, 215, 16, 201, 105, 195, 67, 225, 98, 73, 173, 166, 3], [220, 192, 162, 207, 52, 48, 19, 59, 166, 150, 234, 125, 45, 245, 191, 72, 124, 69, 173, 196, 154, 246, 168, 96, 197, 13, 105, 93, 25, 255, 61, 5]], 
      output_commitments: [[216, 73, 17, 34, 172, 162, 84, 217, 129, 71, 47, 175, 98, 219, 71, 239, 23, 33, 110, 1, 177, 103, 163, 207, 70, 194, 3, 92, 25, 127, 11, 26], [153, 86, 203, 54, 30, 5, 223, 254, 187, 84, 67, 253, 65, 180, 17, 212, 93, 47, 31, 139, 88, 128, 180, 195, 54, 35, 96, 56, 135, 158, 246, 38]], 
      ext_data_hash: [24, 235, 82, 104, 131, 174, 191, 31, 172, 125, 168, 38, 125, 31, 19, 53, 145, 85, 232, 24, 129, 192, 12, 158, 128, 29, 114, 26, 73, 49, 159, 34]
    };
    let depositExtData = { 
      recipient: localjuno.addresses.wallet2, 
      relayer: localjuno.addresses.wallet1, 
      ext_amount: in_ext_amount, 
      fee: in_fee, 
      encrypted_output1: [216, 73, 17, 34, 172, 162, 84, 217, 129, 71, 47, 175, 98, 219, 71, 239, 23, 33, 110, 1, 177, 103, 163, 207, 70, 194, 3, 92, 25, 127, 11, 26], 
      encrypted_output2: [153, 86, 203, 54, 30, 5, 223, 254, 187, 84, 67, 253, 65, 180, 17, 212, 93, 47, 31, 139, 88, 128, 180, 195, 54, 35, 96, 56, 135, 158, 246, 38],
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
      proof: [142, 187, 168, 153, 148, 230, 9, 75, 118, 53, 195, 6, 122, 219, 187, 231, 28, 166, 74, 158, 151, 131, 251, 6, 200, 213, 11, 167, 180, 220, 107, 166, 61, 115, 181, 81, 70, 115, 147, 88, 96, 232, 83, 67, 136, 166, 165, 56, 144, 37, 49, 254, 150, 165, 61, 72, 249, 115, 118, 244, 50, 112, 111, 44, 211, 24, 153, 131, 94, 91, 233, 73, 15, 141, 64, 9, 86, 69, 39, 163, 121, 108, 151, 20, 213, 53, 204, 249, 212, 137, 68, 55, 207, 68, 193, 31, 159, 45, 231, 217, 169, 59, 174, 28, 140, 148, 38, 227, 101, 31, 16, 25, 166, 203, 16, 190, 183, 45, 241, 202, 33, 181, 79, 171, 65, 27, 29, 157], 
      public_amount: [250, 255, 255, 239, 147, 245, 225, 67, 145, 112, 185, 121, 72, 232, 51, 40, 93, 88, 129, 129, 182, 69, 80, 184, 41, 160, 49, 225, 114, 78, 100, 48], 
      roots: [[243, 91, 148, 40, 136, 115, 173, 132, 106, 192, 141, 96, 233, 37, 7, 71, 119, 236, 104, 205, 41, 218, 78, 125, 231, 83, 175, 161, 15, 68, 105, 30], [243, 91, 148, 40, 136, 115, 173, 132, 106, 192, 141, 96, 233, 37, 7, 71, 119, 236, 104, 205, 41, 218, 78, 125, 231, 83, 175, 161, 15, 68, 105, 30]], 
      input_nullifiers: [[123, 214, 54, 194, 153, 133, 188, 106, 30, 163, 6, 72, 16, 251, 37, 217, 101, 48, 92, 85, 217, 245, 248, 88, 205, 240, 136, 12, 22, 218, 228, 24], [167, 162, 248, 103, 249, 55, 57, 171, 179, 64, 89, 135, 160, 243, 193, 196, 15, 54, 124, 231, 18, 152, 182, 45, 178, 21, 111, 186, 233, 234, 48, 2]], 
      output_commitments: [[98, 238, 231, 144, 183, 99, 173, 197, 28, 209, 251, 163, 153, 17, 59, 23, 90, 133, 32, 215, 197, 188, 53, 189, 126, 253, 32, 20, 88, 91, 57, 12], [72, 212, 77, 107, 179, 142, 41, 140, 75, 67, 54, 149, 28, 254, 228, 90, 185, 103, 227, 182, 120, 174, 149, 183, 211, 130, 200, 21, 32, 97, 90, 35]],
      ext_data_hash:  [158, 129, 108, 189, 27, 52, 160, 17, 50, 97, 24, 3, 37, 155, 173, 187, 15, 14, 247, 247, 185, 191, 146, 162, 49, 47, 191, 6, 131, 144, 79, 48]
    };
    let withdrawExtData = { 
      recipient: localjuno.addresses.wallet2, 
      relayer: localjuno.addresses.wallet1, 
      ext_amount: out_ext_amount, 
      fee: out_fee, 
      encrypted_output1: [98, 238, 231, 144, 183, 99, 173, 197, 28, 209, 251, 163, 153, 17, 59, 23, 90, 133, 32, 215, 197, 188, 53, 189, 126, 253, 32, 20, 88, 91, 57, 12], 
      encrypted_output2: [72, 212, 77, 107, 179, 142, 41, 140, 75, 67, 54, 149, 28, 254, 228, 90, 185, 103, 227, 182, 120, 174, 149, 183, 211, 130, 200, 21, 32, 97, 90, 35]
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
  //   1. Wallet3 "wrap"s some CW20 token(ABCT) in anchor
  // ------------------------------------------------
  export async function testVAnchorWrapCw20(
    junod: SigningCosmWasmClient,
    vanchor: string,
    tokenWrapper: string,
    auto: string,
    wallet3: DirectSecp256k1HdWallet,
    auto_amount: string,
  ): Promise<void> {
    process.stdout.write(`Test - Wallet3 wrap ${auto_amount} ABCT in vanchor`);
  
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
  //   1. Wallet3 "unwrap"s some WTW to Cw20 token(ABCT) in vanchor
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