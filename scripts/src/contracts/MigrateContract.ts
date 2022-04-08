import Config from '../config';
import {
  LCDClient,
  isTxError,
  MnemonicKey,
  MsgMigrateContract,
} from "@terra-money/terra.js";

class MigrateContract {

  async set(contract, new_code_id, migrate_msg){
    const typeMessage = 'MigrateContract';

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

    // Construct pool contract
    const msgData = new MsgMigrateContract(
      Config.wallet_address, // admin
      contract,
      new_code_id,
      migrate_msg
     );

    // Sign transaction
    try{
      const tx = await wallet.createAndSignTx({
        msgs: [msgData]
      });

      //Broadcast transaction and check result
      await terra.tx.broadcast(tx).then((txResult) => {
      if (isTxError(txResult)) {
        throw new Error(
          `encountered an error while running the transaction: ${txResult.code} ${txResult.codespace}`
        );
      }
      
      let raw_log = JSON.parse(txResult.raw_log);
        console.log("-----START-----" + typeMessage);
        console.log("hash is: ", txResult.txhash);
        console.log("height is: ", txResult.height);
        let attributes = raw_log[0]['events'][0]['attributes'];
        for ( var i = 0; i < attributes.length; i++ ) {
          if ( attributes[i]["key"] == 'contract_address') {
                console.log("contract_address[" + i + "]: " + attributes[i]["value"]);
            }
        }
        console.log("Logs: ", txResult.logs[0].eventsByType.message);
        console.log("------END-----" + typeMessage);
      });
    }catch(e){
      console.log(e);
    }

  }

}

export = MigrateContract;