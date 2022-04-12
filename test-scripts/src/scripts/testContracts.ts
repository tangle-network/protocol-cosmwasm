import { 
	LCDClient, 
	MnemonicKey, 
	Wallet, 
	MsgExecuteContract
} from "@terra-money/terra.js";
import chalk from "chalk";
import config from "../config";
import { strict as assert } from 'assert';
import { sendTransaction, encodeObjBinary, queryCw20Balance, queryNativeBalance, } from '../utils';

// Variables
let terra: LCDClient;
let deployer: Wallet;

let mixer: string;
let anchor: string;
let vanchor: string;

function initialize() {
    terra = new LCDClient({
        URL: config.networkInfo.lcd_url,
        chainID: config.networkInfo.chainId,
        gasPrices: { uluna: config.networkInfo.gasPrice },
        gasAdjustment: config.networkInfo.gasAdjustment,
    });
	deployer = terra.wallet(new MnemonicKey({ mnemonic: config.mnemonicKeys.deployer }));

	console.log(`Use ${chalk.cyan(deployer.key.accAddress)} as wallet(deployer)`);

	mixer = config.contracts.mixer;
	anchor = config.contracts.anchor;
	vanchor = config.contracts.vanchor;

	console.log(`Use ${chalk.cyan(mixer)} as Mixer`);
	console.log(`Use ${chalk.cyan(anchor)} as Anchor`);
	console.log(`Use ${chalk.cyan(vanchor)} as Vanchor`);
}

export async function testMixer() {
    // Initialize environment info
	console.log("Setup Environment");
	initialize();

    // mixer "DEPOSIT"
    const mixer_deposit_native_msg = {
        "deposit": { 
            "commitment": [60, 193, 57, 161, 207, 107, 11, 192, 51, 187, 64, 70, 168, 216, 155, 216, 187, 112, 123, 6, 14, 101, 174, 89, 250, 120, 41, 24, 101, 151, 110, 24], 
        }
    };
    const fund = { uusd: 1000000 };
	await sendTransaction(terra, deployer, [new MsgExecuteContract(
		deployer.key.accAddress, //sender
		mixer, //contract
		mixer_deposit_native_msg, // ExecMsg to execute contract
		fund
	)]);
	console.log(chalk.green("Mixer deposit Done!")); 

	const recipientBalanceBefore = await queryNativeBalance(
		terra, 
		"uusd", 
		config.contracts.recipient,
	);

    // mixer "WITHDRAW
    const mixer_withdraw_native_msg = {
        "withdraw": { 
            "proof_bytes": [229, 214, 117, 134, 217, 67, 12, 236, 196, 111, 110, 244, 116, 12, 30, 219, 27, 206, 151, 233, 126, 189, 160, 237, 55, 126, 47, 5, 16, 214, 38, 40, 73, 190, 123, 2, 2, 209, 193, 209, 130, 242, 27, 207, 132, 223, 159, 121, 241, 109, 55, 190, 251, 72, 255, 132, 221, 100, 139, 132, 94, 57, 26, 3, 127, 190, 105, 168, 228, 222, 91, 22, 209, 99, 227, 6, 130, 238, 109, 47, 20, 85, 125, 67, 77, 26, 176, 24, 95, 6, 159, 150, 5, 229, 254, 144, 188, 203, 207, 201, 167, 255, 5, 93, 210, 27, 38, 151, 73, 234, 247, 124, 71, 103, 23, 101, 83, 90, 109, 120, 10, 58, 150, 8, 211, 218, 219, 155],
            "root": [82, 25, 2, 85, 65, 173, 18, 5, 74, 175, 108, 14, 232, 197, 174, 9, 242, 59, 105, 48, 104, 169, 204, 128, 253, 150, 15, 102, 108, 214, 81, 33],
            "nullifier_hash": [183, 160, 141, 89, 98, 241, 220, 87, 120, 249, 242, 56, 92, 41, 28, 230, 247, 111, 155, 7, 94, 2, 142, 101, 0, 243, 39, 32, 59, 235, 198, 31],
            "recipient": config.contracts.recipient,
            "relayer": config.contracts.relayer,
            "fee": "0", 
            "refund": "0", 
            "cw20_address": undefined,
        }
    };
    await sendTransaction(terra, deployer, [new MsgExecuteContract(
		deployer.key.accAddress, //sender
		mixer, //contract
		mixer_withdraw_native_msg, // ExecMsg to execute contract
		{} // fund
	)]);

	const recipientBalanceAfter = await queryNativeBalance(
		terra, 
		"uusd", 
		config.contracts.recipient,
	);
	assert.strictEqual(recipientBalanceAfter, recipientBalanceBefore + 1000000);

	console.log(chalk.green("Mixer withdraw Done!"));
}

export async function testAnchor() {
    // Initialize environment info
	console.log("Setup Environment");
	initialize();

    // anchor "DEPOSIT"
    const anchor_deposit_cw20_msg = {
        "deposit_cw20": {
            "commitment": [114, 225, 36, 85, 19, 71, 228, 164, 174, 20, 198, 64, 177, 251, 100, 45, 249, 58, 6, 169, 158, 208, 56, 145, 80, 123, 65, 223, 143, 88, 145, 33]
        }
    };
    const msg = {
        "send": {
            "amount": "1000000",
            "contract": anchor,
            "msg": encodeObjBinary(anchor_deposit_cw20_msg),
        }
    };

	await sendTransaction(terra, deployer, [new MsgExecuteContract(
		deployer.key.accAddress, //sender
		config.contracts.cw20, //contract
		msg, // ExecMsg to execute contract
		{}
	)]);
	console.log(chalk.green("anchor deposit Done!")); 

	const recipientBalanceBefore = await queryCw20Balance(
		terra, 
		config.contracts.cw20, 
		config.contracts.recipient,
	);

    // anchor "WITHDRAW
    const anchor_withdraw_cw20_msg = {
		"withdraw": { 
			"proof_bytes": [90, 249, 64, 247, 109, 43, 39, 43, 127, 147, 229, 67, 15, 213, 234, 24, 187, 126, 198, 37, 194, 70, 161, 33, 62, 18, 134, 53, 129, 165, 5, 10, 168, 232, 41, 122, 186, 111, 104, 142, 47, 66, 50, 172, 97, 255, 75, 254, 11, 254, 30, 154, 158, 24, 149, 136, 232, 227, 166, 90, 154, 212, 3, 39, 30, 20, 127, 166, 129, 102, 51, 233, 7, 46, 39, 179, 184, 10, 32, 148, 194, 253, 52, 33, 176, 125, 46, 157, 117, 52, 208, 18, 212, 0, 151, 136, 102, 212, 236, 123, 36, 167, 9, 133, 186, 37, 128, 123, 240, 179, 90, 33, 173, 96, 94, 98, 147, 11, 62, 131, 179, 3, 221, 162, 149, 147, 49, 160],
			"roots": [[214, 149, 9, 63, 241, 232, 4, 209, 158, 207, 198, 252, 199, 227, 63, 215, 195, 25, 146, 122, 246, 212, 133, 210, 59, 166, 233, 91, 229, 28, 227, 23], [214, 149, 9, 63, 241, 232, 4, 209, 158, 207, 198, 252, 199, 227, 63, 215, 195, 25, 146, 122, 246, 212, 133, 210, 59, 166, 233, 91, 229, 28, 227, 23]],
			"nullifier_hash": [20, 1, 74, 40, 205, 32, 60, 43, 111, 84, 9, 48, 56, 57, 117, 133, 54, 244, 112, 62, 103, 114, 20, 112, 43, 35, 144, 27, 227, 150, 56, 46],
			"recipient": config.contracts.recipient,
			"relayer": "terra17cz29kl6z5wj04ledes9jdmn6pgkelffjxglky",
			"fee": "0", 
			"refund": "0", 
			"commitment": [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
			"cw20_address": config.contracts.cw20,
		}
	};
    await sendTransaction(terra, deployer, [new MsgExecuteContract(
		deployer.key.accAddress, //sender
		anchor, //contract
		anchor_withdraw_cw20_msg, // ExecMsg to execute contract
		{} // fund
	)]);

	const recipientBalanceAfter = await queryCw20Balance(
		terra, 
		config.contracts.cw20, 
		config.contracts.recipient,
	);
	assert.strictEqual(recipientBalanceAfter, recipientBalanceBefore + 1000000);

	console.log(chalk.green("anchor withdraw Done!"));
}
