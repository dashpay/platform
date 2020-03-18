const _ = require('lodash');
/**
 * Return all the utxos (unspendable included)
 * @return {Array}
 */
function getUTXOS(onlyAvailable = true) {
  let utxos = [];

  const self = this;
  const { walletId } = this;
  const subwallets = Object.keys(this.store.wallets[walletId].addresses);
  subwallets.forEach((subwallet) => {
    const paths = Object.keys(self.store.wallets[walletId].addresses[subwallet]);
    paths.forEach((path) => {
      const address = self.store.wallets[walletId].addresses[subwallet][path];
      if (address.utxos) {
        if (!(onlyAvailable && address.locked)) {
          const addrUtxo = address.utxos;
          const addrUtxoIds = Object.keys(addrUtxo);
          if (addrUtxoIds.length > 0) {
            Object.keys(addrUtxo).forEach((utxoid) => {
              const modifiedUtxo = _.cloneDeep(addrUtxo[utxoid]);
              utxos = utxos.concat(modifiedUtxo);
            });
          }
        }
      }
    });
  });
  utxos = utxos.sort((a, b) => b.satoshis - a.satoshis);

  return utxos;
}
module.exports = getUTXOS;
