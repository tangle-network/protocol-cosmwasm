import { CosmWasmClient, SigningCosmWasmClient } from "@cosmjs/cosmwasm-stargate";
import { Coin, coin, DirectSecp256k1HdWallet, DirectSignResponse, makeAuthInfoBytes, makeSignDoc } from "@cosmjs/proto-signing";
import { GasPrice } from "@cosmjs/stargate";

import * as chai from "chai";
import chaiAsPromised from "chai-as-promised";
import chalk from "chalk";
import { BigNumber, BigNumberish, ethers } from "ethers";
import keccak256 from "keccak256";
import BN from 'bn.js';
import EC from 'elliptic';

import { localjuno } from "../../config/localjunoConstants";
import { datetimeStringToUTC,toEncodedBinary } from "../../utils/helpers";

chai.use(chaiAsPromised);
const { expect } = chai;
const ec = new EC.ec('secp256k1');

const SET_RESOURCE_FUNCTION_NAME ="adminSetResourceWithSignature(bytes32,bytes4,uint32,bytes32,address,address,bytes)";

// -----------------------------------------------
//  TEST: signatureBridge
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

    const left_pad_buf: Buffer = Buffer.alloc(6);

    const chainIdType: number = getChainIdType(parseInt(localjuno.contractsConsts.chain_id, 10));
    const chainIdType_buf: Buffer = Buffer.allocUnsafe(6);
    chainIdType_buf.writeUintBE(chainIdType, 0, 6);
    
    // const resource_id: string = toFixedHex(Buffer.from(hashed_sigbridge_addr), 26) + toHex(chainIdType, 6).substring(2);
    const hashed_sigbridge_addr_buf: Buffer = Buffer.from(keccak256(signatureBridge).buffer.slice(12))
    const resource_id: Buffer = Buffer.concat([left_pad_buf, hashed_sigbridge_addr_buf, chainIdType_buf]);
    
    // const new_resource_id: string = toFixedHex(Buffer.from(hashed_anchor_address), 26) + toHex(chainIdType, 6).substring(2);
    const hashed_anchor_addr_buf: Buffer = Buffer.from(keccak256(localjuno.contracts.anchor).buffer.slice(12))
    const new_resource_id: Buffer = Buffer.concat([left_pad_buf, hashed_anchor_addr_buf, chainIdType_buf]);

    const function_sig = Buffer.from(keccak256(SET_RESOURCE_FUNCTION_NAME).buffer.slice(0, 4));

    const handler_addr = localjuno.contracts.anchorHandler;
    const execution_context_addr = localjuno.contracts.anchor;

    const unsignedData = Buffer.concat([
        resource_id,
        function_sig,
        nonce_buf,
        new_resource_id,
        Buffer.from(handler_addr),
        Buffer.from(execution_context_addr),
    ]); 

    const privkey = localjuno.contractsConsts.testPrivKey;

    // const hashed_unsignedData = keccak256(unsignedData);
    // console.log("hashed_unsigned_data::::", Array.from(hashed_unsignedData), "\n");
    // const [account1, account2] = await wallet1.getAccounts();
    // const accountNumber = await (await junod.getAccount(account1.address))?.accountNumber;
    // const sequence = await (await junod.getAccount(account1.address))?.sequence!;
    // const signed_data: DirectSignResponse = await wallet1.signDirect(
    //     localjuno.addresses.wallet1, 
    //     makeSignDoc(
    //         hashed_unsignedData, 
    //         makeAuthInfoBytes(
    //             [{
    //                 pubkey: account1.pubkey as any,
    //                 sequence: sequence,
    //             }], 
    //             [], 
    //             1.3, 
    //             2,
    //         ),
    //         localjuno.networkInfo.chainId, 
    //         accountNumber!,
    //     )
    // );

    const sig = signMessage(privkey, Array.from(unsignedData));
    console.log(sig);

    // console.log("account1.pubkey:", account1.pubkey);
    // console.log("signature:::", Array.from(Buffer.from(signed_data.signature.signature, 'base64')), "\n");
    // const x = {  
    //     resource_id: Array.from(resource_id),
    //     function_sig: Array.from(function_sig),
    //     nonce: nonce,
    //     new_resource_id: Array.from(new_resource_id),
    //     handler_addr: handler_addr,
    //     execution_context_addr: execution_context_addr,
    //     sig: Array.from(Buffer.from(sig.substring(2), 'hex')),
    // }
    // console.log("X::::", x, "\n");
    const result = await junod.execute(localjuno.addresses.wallet1, signatureBridge, {
        admin_set_resource_with_sig: {  
            resource_id: Array.from(resource_id),
            function_sig: Array.from(function_sig),
            nonce: nonce,
            new_resource_id: Array.from(new_resource_id),
            handler_addr: handler_addr,
            execution_context_addr: execution_context_addr,
            sig: Array.from(Buffer.from(sig.substring(2), 'hex')),
        },
    },
    "auto", undefined, []);

    // Check the result
    const contractAddrQuery: any = await junod.queryContractSmart(localjuno.contracts.anchorHandler, {
        get_resource_id: new_resource_id,
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

    
    const left_pad_buf: Buffer = Buffer.alloc(6);

    const chainIdType: number = getChainIdType(parseInt(localjuno.contractsConsts.chain_id, 10));
    const chainIdType_buf: Buffer = Buffer.allocUnsafe(6);
    chainIdType_buf.writeUintBE(chainIdType, 0, 6);

    const hashed_anchor_addr_buf: Buffer = Buffer.from(keccak256(localjuno.contracts.anchor).buffer.slice(12))
    const resource_id: Buffer = Buffer.concat([left_pad_buf, hashed_anchor_addr_buf, chainIdType_buf]);

    // proposal of `update_edge` with mock edge info
    const data = Buffer.concat([
        resource_id,
        Buffer.from(toEncodedBinary({
            update_edge: {
                src_chain_id: 100,
                root: [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
                latest_leaf_index: 1,
                target: [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
            }
        })),
    ]);
    // const unsignedData = keccak256(data); // Currently, it is "0x..." + "(base64 string)".
    // const signed_data: DirectSignResponse = await wallet1.signDirect(
    //     localjuno.addresses.wallet1, 
    //     makeSignDoc(
    //         Buffer.from(unsignedData), 
    //         makeAuthInfoBytes([], [], 1.3),
    //         localjuno.networkInfo.chainId, 
    //         1
    //     )
    // );

    const privKey = localjuno.contractsConsts.testPrivKey;
    const sig = signMessage(privKey, data);
    // console.log("unsigned_data:::", unsignedData);
    // console.log("signed-data:::", signed_data);
    // const x = {
    //     exec_proposal_with_sig: {
    //         data: Array.from(Buffer.from(data)),
    //         sig: Array.from(Buffer.from(signed_data.signature.signature, 'base64')),
    //     }
    // };
    // console.log("x:::", x);

    const result = await junod.execute(localjuno.addresses.wallet1, signatureBridge, {
      exec_proposal_with_sig: {
        data: Array.from(data),
        // sig: Array.from(Buffer.from(signed_data.signature.signature, 'base64')),
        sig: Array.from(Buffer.from(sig)),
      }
    },
    "auto", undefined, []);

    // Check the result
    const edgeInfoQuery: any = await junod.queryContractSmart(localjuno.contracts.anchor, {
        edge_info: {
            id: 100,
        },
    });
    expect(edgeInfoQuery.latest_leaf_index == 1).to.be.ok;

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

