import Config from '../config';
import {
  LCDClient,
  isTxError,
  MnemonicKey,
  MsgInstantiateContract,
} from "@terra-money/terra.js";

class QueryContract {

  async set(contract_address, query_msg){
    const typeMessage = 'QueryContract';

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

    console.log("------START-----" + typeMessage);
    try{
        const queryResult = await terra.wasm.contractQuery(
            contract_address,
            query_msg
        );
        console.log(`Contract: ${contract_address}`);
        console.log(query_msg);
        console.log("Result::");
        console.log(queryResult);
    }catch(e){
      console.log(e);
    }
    console.log("------END-----" + typeMessage);
  }

}

export = QueryContract;
