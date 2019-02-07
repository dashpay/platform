const { Worker } = require('../../src/plugins');

class ColdStorageWorker extends Worker {
  constructor(props) {
    super({
      executeOnStart: true,
      workerIntervalTime: 6 * 60 * 60 * 1000,
      dependencies: [
        'walletConsolidator',
        'getUTXOS',
        'getBalance',
      ],
    });
    if (!props.address) {
      return new Error('ColdStorageWorker expect an address');
    }
    this.address = props.address;
  }

  execute() {
    const { walletConsolidator } = this;
    const utxos = this.getUTXOS();
    if (utxos.length === 0) {
      console.error('ColdStorageWorker : We did not found any utxos. Doing nothing');
    } else {
      const balance = this.getBalance();
      console.log('Found inputs to move');
      const consolidate = walletConsolidator.consolidateWallet(this.address, utxos);
      const preparedTransaction = consolidate.prepareTransaction();
      const rawTx = preparedTransaction.toString();
      preparedTransaction
        .broadcast()
        .then((txid) => {
          console.log('Worker has moved ', balance, 'txid:', txid, 'rawTx:', rawTx);
        });
    }
    console.log('Next execution in 6 hours.');
  }
}
module.exports = ColdStorageWorker;
