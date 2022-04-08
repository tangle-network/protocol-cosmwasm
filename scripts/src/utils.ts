import { 
	LCDClient, 
	Wallet, 
	isTxError, 
	Msg, 
	MsgInstantiateContract, 
	MsgStoreCode, 
} from "@terra-money/terra.js";

import axios from "axios";
import chalk from "chalk";
import * as fs from "fs";

/**
 * @notice Upload contract code to LocalTerra. Return code ID.
 */
 export async function storeCode(
    terra: LCDClient,
    deployer: Wallet,
    filepath: string
): Promise<number> {
    const code = fs.readFileSync('./wasm_contracts/' + filepath).toString("base64");
    const result = await sendTransaction(terra, deployer, [
        new MsgStoreCode(deployer.key.accAddress, code),
    ]);
    return parseInt(result.logs[0].eventsByType.store_code.code_id[0]);
}

/**
 * @notice Instantiate a contract from an existing code ID. Return contract address.
 */
// eslint-disable-next-line @typescript-eslint/explicit-module-boundary-types
export async function instantiateContract(
    terra: LCDClient,
    deployer: Wallet,
    admin: Wallet, // leave this emtpy then contract is not migratable
    codeId: number,
    instantiateMsg: Record<string, unknown>
  ) {
    const result = await sendTransaction(terra, deployer, [
      new MsgInstantiateContract(
        deployer.key.accAddress,
        admin.key.accAddress,
        codeId,
        instantiateMsg
      ),
    ]);
    return result;
  }


/**
 * @notice Send a transaction. Return result if successful, throw error if failed.
 */
// eslint-disable-next-line @typescript-eslint/explicit-module-boundary-types
export async function sendTransaction(
    terra: LCDClient,
    sender: Wallet,
    msgs: Msg[],
    verbose = false
  ): Promise<any> {
    const tx = await sender.createAndSignTx({ msgs });
    const result = await terra.tx.broadcast(tx);
  
    // Print the log info
    if (verbose) {
      console.log(chalk.magenta("\nTxHash:"), result.txhash);
      try {
        console.log(
          chalk.magenta("Raw log:"),
          JSON.stringify(JSON.parse(result.raw_log), null, 2)
        );
      } catch {
        console.log(chalk.magenta("Failed to parse log! Raw log:"), result.raw_log);
      }
    }
  
    if (isTxError(result)) {
      throw new Error(
        chalk.red("Transaction failed!") +
          `\n${chalk.yellow("code")}: ${result.code}` +
          `\n${chalk.yellow("codespace")}: ${result.codespace}` +
          `\n${chalk.yellow("raw_log")}: ${result.raw_log}`
      );
    }
  
    return result;
  }
  
export function encodeObjBinary(obj: any) {
    return Buffer.from(JSON.stringify(obj)).toString("base64");
} 