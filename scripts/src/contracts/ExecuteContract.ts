import Config from '../config';
import {
  LCDClient,
  isTxError,
  MnemonicKey,
  MsgInstantiateContract,
  MsgExecuteContract,
} from "@terra-money/terra.js";

class ExecuteContract {

  async set(contract_address, execute_msg, fund){
    const typeMessage = 'ExecuteContract';

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

    const execute = new MsgExecuteContract(
      wallet.key.accAddress, //sender
      contract_address, //contract
      execute_msg, // ExecMsg to execute contract
      fund
    );
    
    console.log("------START-----" + typeMessage);
    try{
        const executeTx = await wallet.createAndSignTx({
          msgs: [execute],
        });
        const executeTxResult = await terra.tx.broadcast(executeTx);
        
        console.log(executeTxResult);
        
        if (isTxError(executeTxResult)) {
          throw new Error(
            `instantiate failed. code: ${executeTxResult.code}, codespace: ${executeTxResult.codespace}, raw_log: ${executeTxResult.raw_log}`
          );
        }
        
        const {
          instantiate_contract: { contract_address },
        } = executeTxResult.logs[0].eventsByType;
    }catch(e){
      console.log(e);
    }
    console.log("------END-----" + typeMessage);
  }

}

export = ExecuteContract;
