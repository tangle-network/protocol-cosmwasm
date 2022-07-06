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
//  TEST: Treasury contract
//  
//  SCENARIO: 
//   1. Initialize the "Treasury" contract (already done in "setup")
//   2. Check if the state/config matches the setup input
// ------------------------------------------------
export async function testTreasuryInitialize(
    junod: SigningCosmWasmClient,
    Treasury: string,
  ): Promise<void> {
    process.stdout.write("Test - Treasury should initialize");
    const result: any = await junod.queryContractSmart(Treasury, {
      get_config: {},
    });
  
    expect(result.treasury_handler == localjuno.contracts.treasuryHandler).to.be.ok;
    
    console.log(chalk.green(" Passed!"));
  }
