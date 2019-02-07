const { StandardPlugin } = require('../../src/plugins/index');

class WalletConsolidator extends StandardPlugin {
  constructor() {
    super({
      // When true, the wallet object will only fire "ready"
      firstExecutionRequired: false,
      // Describe if we execute it first on startup of an account.
      executeOnStart: false,
      // Methods and function that we would want to use
      dependencies: [
        'getUTXOS',
        'getUnusedAddress',
        'getBalance',
        'createTransactionFromUTXOS',
        'broadcastTransaction',
      ],
    });
  }

  consolidateWallet(address = this.getUnusedAddress().address, utxos = this.getUTXOS()) {
    const self = this;
    return {
      prepareTransaction: () => {
        if (!utxos || utxos.length === 0) {
          throw new Error('There is nothing to consolidate');
        }
        const opts = {
          utxos,
          recipient: address,
        };

        const rawtx = this.createTransactionFromUTXOS(opts);
        return {
          toString: () => rawtx,
          broadcast: async () => {
            console.log('TRIED TO BROADCAST', rawtx);
            return rawtx;
            // return self.broadcastTransaction(rawtx);
          },
        };
      },
    };
  }
}
module.exports = WalletConsolidator;
