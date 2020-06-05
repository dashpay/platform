const { Address, Transaction } = require('@dashevo/dashcore-lib');

/**
 * Return all the utxos
 * @return {UnspentOutput[]}
 */
function getUTXOS() {
  const utxos = [];

  const self = this;
  const { walletId, network } = this;
  /* eslint-disable-next-line no-restricted-syntax */
  for (const walletType in this.store.wallets[walletId].addresses) {
    if (walletType && ['external', 'internal', 'misc'].includes(walletType)) {
      /* eslint-disable-next-line no-restricted-syntax */
      for (const path in self.store.wallets[walletId].addresses[walletType]) {
        if (path) {
          const address = self.store.wallets[walletId].addresses[walletType][path];
          /* eslint-disable-next-line no-restricted-syntax */
          for (const identifier in address.utxos) {
            if (identifier) {
              const [txid, outputIndex] = identifier.split('-');

              utxos.push(new Transaction.UnspentOutput(
                {
                  txId: txid,
                  vout: parseInt(outputIndex, 10),
                  script: address.utxos[identifier].script,
                  satoshis: address.utxos[identifier].satoshis,
                  address: new Address(address.address, network),
                },
              ));
            }
          }
        }
      }
    }
  }
  return utxos.sort((a, b) => b.satoshis - a.satoshis);
}

module.exports = getUTXOS;
