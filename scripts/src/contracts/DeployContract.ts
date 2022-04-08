import Config from '../config';
import {
  LCDClient,
  isTxError,
  MnemonicKey,
  MsgStoreCode,
} from "@terra-money/terra.js";
import * as fs from 'fs';

class DeployContract {

  async set(file_name){
    const typeMessage = 'DeployContract';

    //LCD Config
    const terra = new LCDClient({
      URL: Config.lcd_url,
      chainID: Config.chaindId,
      gasPrices: { uluna: Config.gasPrice },
      gasAdjustment: Config.gasAdjustment
    });
    //Wallet Seed
    const mk = new MnemonicKey({
      mnemonic: Config.wallet_seed,
    });
    const wallet = terra.wallet(mk);

    const storeCode = new MsgStoreCode(
      wallet.key.accAddress,
      fs.readFileSync('./wasm_contracts/' + file_name).toString('base64')
    );

    console.log("-----START-----" + typeMessage + " - " + file_name);
    try{
        const storeCodeTx = await wallet.createAndSignTx({
          msgs: [storeCode],
        });
        const storeCodeTxResult = await terra.tx.broadcast(storeCodeTx);
        
        
        console.log(storeCodeTxResult);
        
        
        if (isTxError(storeCodeTxResult)) {
          throw new Error(
            `store code failed. code: ${storeCodeTxResult.code}, codespace: ${storeCodeTxResult.codespace}, raw_log: ${storeCodeTxResult.raw_log}`
          );
        }
        
        const {
          store_code: { code_id },
        } = storeCodeTxResult.logs[0].eventsByType;
    }catch(e){
      console.log(e);
    }
    console.log("------END-----" + typeMessage + " - " + file_name);
  }

}

export = DeployContract;
