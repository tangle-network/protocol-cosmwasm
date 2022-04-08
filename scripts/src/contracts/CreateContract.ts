import Config from '../config';
import {
  LCDClient,
  isTxError,
  MnemonicKey,
  MsgInstantiateContract,
} from "@terra-money/terra.js";

class CreateContract {

  async set(code_id, execute_msg){
    const typeMessage = 'CreateContract';

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

    const instantiate = new MsgInstantiateContract(
      wallet.key.accAddress, //sender
      wallet.key.accAddress, //admin
      code_id, // code ID of deployed contract
      execute_msg, // InitMsg to execute contract
      //{ uluna: 1000000 }
    );
    
    console.log("------START-----" + typeMessage);
    try{
        const instantiateTx = await wallet.createAndSignTx({
          msgs: [instantiate],
        });
        const instantiateTxResult = await terra.tx.broadcast(instantiateTx);
        
        console.log(instantiateTxResult);
        
        if (isTxError(instantiateTxResult)) {
          throw new Error(
            `instantiate failed. code: ${instantiateTxResult.code}, codespace: ${instantiateTxResult.codespace}, raw_log: ${instantiateTxResult.raw_log}`
          );
        }
        
        const {
          instantiate_contract: { contract_address },
        } = instantiateTxResult.logs[0].eventsByType;
    }catch(e){
      console.log(e);
    }
    console.log("------END-----" + typeMessage);
  }

}

export = CreateContract;
