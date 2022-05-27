/* eslint-disable @typescript-eslint/no-unused-vars */
/* eslint-disable @typescript-eslint/no-explicit-any */
import * as path from "path";
import chalk from "chalk";
import {
  storeCode,
  migrateContract,
} from "../../utils/helpers";
import { wasm_path } from "../../config/wasmPaths";
import { SigningCosmWasmClient } from "@cosmjs/cosmwasm-stargate";
import { DirectSecp256k1HdWallet } from "@cosmjs/proto-signing";


// -----------------------------
// Base functions to migrate contracts with
// -----------------------------
export async function migrateContracts(
    junod: SigningCosmWasmClient,
    wallet1: DirectSecp256k1HdWallet,
    fundsRouter: string,
    timeConditions: string,
  ): Promise<void> {
    // run the migrations desired
  }
  

  