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
//  TEST: TokenWrapper
//  
//  SCENARIO: 
//   1. Initialize the "(Governed)TokenWrapper" contract (already done in "setup")
//   2. Check if the state/config matches the setup input
// ------------------------------------------------
export async function testTokenWrapperInitialize(
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
  