import { SigningCosmWasmClient } from "@cosmjs/cosmwasm-stargate";
import { DirectSecp256k1HdWallet } from "@cosmjs/proto-signing";

import * as chai from "chai";
import chaiAsPromised from "chai-as-promised";
import chalk from "chalk";
import { BigNumber, BigNumberish, ethers } from "ethers";
import keccak256 from "keccak256";
import BN from 'bn.js';
import EC from 'elliptic';

import { localjuno } from "../../config/localjunoConstants";
import { toEncodedBinary } from "../../utils/helpers";

chai.use(chaiAsPromised);
const { expect } = chai;
const ec = new EC.ec('secp256k1');

// -----------------------------------------------
//  TEST: SignatureBridge
//  
//  SCENARIO: 
//   1. Initialize the "SignatureBridge" contract (already done in "setup")
//   2. Check if the state/config matches the setup input
// ------------------------------------------------
export async function testSignatureBridgeInitialize(
    junod: SigningCosmWasmClient,
    signatureBridge: string,
  ): Promise<void> {
    process.stdout.write("Test - SignatureBridge should initialize");
    const result: any = await junod.queryContractSmart(signatureBridge, {
      get_state: {},
    });
  
    expect(result.proposal_nonce == 0).to.be.ok;

    console.log(chalk.green(" Passed!"));
  }
  

// -----------------------------------------------
//  TEST: SignatureBridge
//  
//  SCENARIO: 
//   Governor(admin) sets the resource with signature.
// ------------------------------------------------
export async function testSignatureBridgeAdminSetResWithSignature(
    junod: SigningCosmWasmClient,
    wallet1: DirectSecp256k1HdWallet,
    signatureBridge: string,
): Promise<void> {
    process.stdout.write("Test - SignatureBridge admin sets the resource with signature");

    const stateQuery: any = await junod.queryContractSmart(signatureBridge, {
        get_state: {},
    });
    const before_nonce = stateQuery.proposal_nonce;


    const nonce: number = before_nonce + 1;
    const nonce_buf: Buffer = Buffer.allocUnsafe(4);
    nonce_buf.writeUInt32BE(nonce);

    const resource_id: Buffer = genResourceId(signatureBridge);
    const function_sig = Buffer.alloc(4);
    const new_resource_id: Buffer = genResourceId(localjuno.contracts.anchor)
    const handler_addr = localjuno.contracts.anchorHandler;
    const execution_context_addr = localjuno.contracts.anchor;

    const data = Buffer.from(toEncodedBinary(
      {
        resource_id: Array.from(resource_id),
        function_sig: Array.from(function_sig),
        nonce: nonce,
        new_resource_id: Array.from(new_resource_id),
        handler_addr: handler_addr,
        execution_context_addr: execution_context_addr,
      }
    ), 'base64');

    const privkey = localjuno.contractsConsts.testPrivKey;
    const sig = signMessage(privkey, Array.from(data));
    const sigLen = sig.length;

    const result = await junod.execute(localjuno.addresses.wallet1, signatureBridge, {
        admin_set_resource_with_sig: {  
            data: Array.from(data),
            sig: Array.from(Buffer.from(sig.substring(2, sigLen - 2), 'hex')),
        },
    },
    "auto", undefined, []);

    // Check the result
    const contractAddrQuery: any = await junod.queryContractSmart(localjuno.contracts.anchorHandler, {
        get_contract_address: {
          resource_id: Array.from(new_resource_id),
        }
    });
    const contract_addr = contractAddrQuery.contract_addr;

    expect(contract_addr == execution_context_addr).to.be.ok;
    console.log(chalk.green(" Passed!"));
}


// -----------------------------------------------
//  TEST: SignatureBridge
//  
//  SCENARIO: 
//   Governor(admin) sets the resource with signature.
// ------------------------------------------------
export async function testSignatureBridgeExecProposalWithSignature(
    junod: SigningCosmWasmClient,
    wallet1: DirectSecp256k1HdWallet,
    signatureBridge: string,
): Promise<void> {
    process.stdout.write("Test - SignatureBridge admin executes the proposal with signature");

    // Save the before-state
    let configQuery = await junod.queryContractSmart(localjuno.contracts.anchor, {
      config: {},
    });
    const beforeNonce: number = configQuery.proposal_nonce;

    // proposal of `set_handler`
    const resource_id: Buffer = genResourceId(localjuno.contracts.anchor);
    const data = Buffer.concat([
        resource_id,
        Buffer.from(toEncodedBinary({
          set_handler: {
            handler: localjuno.contracts.anchorHandler,
            nonce: beforeNonce + 3,
          }
        }), 'base64'),
      ]);

    const privKey = localjuno.contractsConsts.testPrivKey;
    const sig = signMessage(privKey, data);
    const sigLen = sig.length;

    const result = await junod.execute(localjuno.addresses.wallet1, signatureBridge, {
      exec_proposal_with_sig: {
        data: Array.from(data),
        sig: Array.from(Buffer.from(sig.substring(2, sigLen - 2), 'hex')),
      }
    },
    "auto", undefined, []);

    // Check the result
    configQuery = await junod.queryContractSmart(localjuno.contracts.anchor, {
      config: {},
    });
    const afterNonce: number = configQuery.proposal_nonce;
    expect(afterNonce == beforeNonce + 3).to.be.ok;

    console.log(chalk.green(" Passed!"));
}

/** BigNumber to hex string of specified length */
export function toFixedHex(number: BigNumberish, length: number = 32): string {
    let result =
      '0x' +
      (number instanceof Buffer
        ? number.toString('hex')
        : BigNumber.from(number.toString()).toHexString().replace('0x', '')
      ).padStart(length * 2, '0')
    if (result.indexOf('-') > -1) {
      result = '-' + result.replace('-', '')
    }
    return result
}

/**
 * Computes the updated chain ID with chain type.
 * @param chainID Chain ID to encode into augmented chain ID Type, defaults to hardhat's chain ID.
 * @returns 
 */
export const getChainIdType = (chainID: number = 1): number => {
    const CHAIN_TYPE = '0x0400';
    const chainIdType = CHAIN_TYPE + toFixedHex(chainID, 4).substr(2);
    return Number(BigInt(chainIdType));
}
  
export const toHex = (covertThis: ethers.utils.BytesLike | number | bigint, padding: number): string => {
    return ethers.utils.hexZeroPad(ethers.utils.hexlify(covertThis), padding);
};

export const signMessage = (privKey: string, data: any) => {
    const key = ec.keyFromPrivate(privKey.slice(2), 'hex');
    const hash = ethers.utils.keccak256(data);
    const hashedData = ethers.utils.arrayify(hash); 
    let signature = key.sign(hashedData)!;
    let expandedSig = {
      r: '0x' + signature.r.toString('hex'),
      s: '0x' + signature.s.toString('hex'),
      v: signature.recoveryParam! + 27,
    }
    let sig;
    // Transaction malleability fix if s is too large (Bitcoin allows it, Ethereum rejects it)
    try {
      sig = ethers.utils.joinSignature(expandedSig)
    } catch (e) {
      expandedSig.s = '0x' + (new BN(ec.curve.n).sub(signature.s)).toString('hex');
      expandedSig.v = (expandedSig.v === 27) ? 28 : 27;
      sig = ethers.utils.joinSignature(expandedSig)
    }
  
    return sig;
  };

const genResourceId = (address: string): Buffer => {
    const leftPadBuf: Buffer = Buffer.alloc(6);

    const hashedAddrBuf: Buffer = Buffer.from(keccak256(address).buffer.slice(12))

    const chainIdType: number = getChainIdType(parseInt(localjuno.contractsConsts.chain_id, 10));
    const chainIdType_buf: Buffer = Buffer.allocUnsafe(6);
    chainIdType_buf.writeUintBE(chainIdType, 0, 6);
    
    const resource_id: Buffer = Buffer.concat([leftPadBuf, hashedAddrBuf, chainIdType_buf]);

    return resource_id;
}


