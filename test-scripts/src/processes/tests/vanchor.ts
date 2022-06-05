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
      proof: [30, 112, 87, 195, 127, 201, 224, 192, 203, 17, 114, 63, 206, 53, 246, 44, 154, 53, 158, 151, 253, 88, 92, 134, 34, 231, 107, 190, 40, 1, 4, 143, 99, 243, 130, 31, 235, 198, 145, 24, 132, 255, 251, 85, 34, 75, 53, 120, 252, 221, 217, 25, 139, 104, 50, 252, 128, 74, 184, 42, 181, 230, 31, 1, 76, 34, 8, 226, 182, 246, 242, 123, 1, 180, 107, 59, 88, 235, 249, 127, 254, 161, 41, 2, 236, 186, 241, 152, 51, 126, 153, 77, 96, 45, 20, 130, 51, 19, 204, 125, 56, 81, 159, 251, 154, 252, 34, 146, 151, 60, 39, 216, 165, 26, 63, 208, 249, 193, 173, 154, 128, 253, 241, 209, 217, 55, 24, 146], 
      public_amount: [10, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0], 
      roots: [[239, 89, 25, 14, 35, 42, 26, 61, 180, 140, 224, 106, 63, 122, 122, 78, 89, 65, 28, 26, 74, 63, 65, 52, 176, 152, 45, 245, 107, 212, 24, 9], [239, 89, 25, 14, 35, 42, 26, 61, 180, 140, 224, 106, 63, 122, 122, 78, 89, 65, 28, 26, 74, 63, 65, 52, 176, 152, 45, 245, 107, 212, 24, 9]], 
      input_nullifiers: [[224, 165, 53, 18, 173, 250, 1, 6, 116, 81, 253, 141, 161, 15, 154, 235, 13, 168, 140, 246, 212, 77, 231, 208, 96, 80, 228, 184, 162, 53, 88, 32], [189, 77, 100, 41, 230, 177, 166, 236, 13, 175, 89, 198, 212, 179, 74, 166, 251, 129, 72, 74, 184, 110, 171, 89, 195, 255, 7, 222, 136, 120, 144, 43]], 
      output_commitments: [[51, 170, 43, 18, 241, 27, 148, 191, 180, 66, 129, 121, 76, 55, 72, 92, 154, 5, 174, 234, 119, 175, 22, 78, 12, 231, 3, 94, 147, 32, 125, 32], [118, 160, 222, 87, 203, 95, 19, 112, 183, 150, 179, 138, 42, 218, 199, 64, 60, 80, 99, 161, 160, 78, 146, 254, 116, 117, 190, 81, 39, 128, 55, 1]], 
      ext_data_hash: [185, 233, 54, 0, 212, 197, 132, 236, 29, 79, 18, 216, 69, 52, 78, 96, 245, 63, 47, 2, 227, 107, 23, 103, 100, 185, 119, 75, 189, 139, 220, 36]
    };
    let depositExtData = { 
      recipient: localjuno.addresses.wallet2, 
      relayer: localjuno.addresses.wallet1, 
      ext_amount: in_ext_amount, 
      fee: in_fee, 
      encrypted_output1: [51, 170, 43, 18, 241, 27, 148, 191, 180, 66, 129, 121, 76, 55, 72, 92, 154, 5, 174, 234, 119, 175, 22, 78, 12, 231, 3, 94, 147, 32, 125, 32], 
      encrypted_output2: [118, 160, 222, 87, 203, 95, 19, 112, 183, 150, 179, 138, 42, 218, 199, 64, 60, 80, 99, 161, 160, 78, 146, 254, 116, 117, 190, 81, 39, 128, 55, 1],
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
      proof: [6, 15, 218, 177, 117, 235, 114, 126, 18, 191, 5, 117, 117, 130, 27, 242, 105, 237, 167, 173, 58, 16, 55, 33, 22, 202, 52, 118, 250, 128, 24, 141, 123, 25, 69, 125, 205, 228, 45, 63, 243, 107, 146, 122, 192, 81, 229, 170, 104, 109, 151, 55, 18, 54, 89, 25, 77, 139, 13, 112, 40, 148, 59, 2, 96, 138, 0, 106, 249, 75, 59, 151, 41, 52, 36, 36, 174, 97, 57, 236, 48, 33, 139, 240, 120, 57, 202, 196, 107, 99, 19, 72, 142, 17, 163, 34, 165, 23, 199, 90, 53, 243, 220, 183, 148, 172, 5, 171, 114, 255, 223, 40, 128, 55, 76, 74, 239, 212, 130, 143, 191, 153, 50, 226, 42, 18, 36, 150], 
      public_amount: [250, 255, 255, 239, 147, 245, 225, 67, 145, 112, 185, 121, 72, 232, 51, 40, 93, 88, 129, 129, 182, 69, 80, 184, 41, 160, 49, 225, 114, 78, 100, 48], 
      roots: [[217, 20, 52, 153, 71, 108, 157, 246, 118, 136, 162, 174, 179, 11, 230, 202, 111, 243, 7, 194, 127, 104, 188, 72, 177, 192, 208, 73, 30, 17, 193, 16], [217, 20, 52, 153, 71, 108, 157, 246, 118, 136, 162, 174, 179, 11, 230, 202, 111, 243, 7, 194, 127, 104, 188, 72, 177, 192, 208, 73, 30, 17, 193, 16]], 
      input_nullifiers: [[21, 220, 15, 156, 175, 19, 142, 182, 60, 133, 175, 250, 219, 252, 100, 173, 28, 42, 252, 248, 19, 57, 244, 112, 62, 113, 153, 19, 251, 16, 121, 40], [28, 89, 229, 192, 84, 147, 99, 165, 199, 33, 194, 56, 4, 74, 56, 163, 1, 25, 124, 188, 10, 70, 47, 147, 127, 165, 42, 197, 103, 113, 103, 7]], 
      output_commitments: [[125, 110, 224, 1, 48, 212, 222, 90, 24, 185, 247, 184, 195, 136, 3, 31, 194, 241, 204, 74, 166, 241, 108, 174, 60, 53, 231, 172, 185, 73, 238, 29], [187, 171, 58, 24, 2, 94, 165, 218, 55, 45, 129, 76, 99, 82, 221, 82, 249, 161, 176, 31, 55, 122, 23, 232, 184, 113, 110, 234, 202, 38, 36, 32]],
      ext_data_hash: [20, 180, 8, 58, 78, 250, 172, 0, 115, 37, 109, 166, 113, 238, 248, 93, 84, 147, 18, 87, 48, 95, 194, 125, 85, 53, 110, 166, 201, 15, 24, 0]
    };
    let withdrawExtData = { 
      recipient: localjuno.addresses.wallet2, 
      relayer: localjuno.addresses.wallet1, 
      ext_amount: out_ext_amount, 
      fee: out_fee, 
      encrypted_output1: [125, 110, 224, 1, 48, 212, 222, 90, 24, 185, 247, 184, 195, 136, 3, 31, 194, 241, 204, 74, 166, 241, 108, 174, 60, 53, 231, 172, 185, 73, 238, 29], 
      encrypted_output2: [187, 171, 58, 24, 2, 94, 165, 218, 55, 45, 129, 76, 99, 82, 221, 82, 249, 161, 176, 31, 55, 122, 23, 232, 184, 113, 110, 234, 202, 38, 36, 32]
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