/* eslint-disable no-continue, no-restricted-syntax */
const { Address, Transaction } = require('@dashevo/dashcore-lib');
const { COINBASE_MATURITY } = require('../../../CONSTANTS');

/**
 * Return all the utxos
 * @param {getUTXOSOptions} options - Options object
 * @param {Number} [options.coinbaseMaturity] - Allow to override coinbase maturity
 * @return {UnspentOutput[]}
 */
function getUTXOS(options = {
  coinbaseMaturity: COINBASE_MATURITY,
}) {
  const {
    walletId,
    network,
  } = this;

  const utxos = [];

  const chainStore = this.storage.getChainStore(network);
  const accountState = this.storage.getWalletStore(walletId).getPathState(this.accountPath);
  const currentBlockHeight = chainStore.state.chainHeight;

  Object.values(accountState.addresses).forEach((address) => {
    const addressData = chainStore.getAddress(address);
    const utxosKeys = Object.keys(addressData.utxos);

    utxosKeys.forEach((utxoIdentifier) => {
      let skipUtxo = false;
      const [txid, outputIndex] = utxoIdentifier.split('-');

      const txInStore = chainStore.getTransaction(txid);

      if (txInStore && txInStore.transaction.isCoinbase()) {
        const { transaction, metadata } = txInStore;

        let transactionHeight;
        if (metadata) {
          transactionHeight = metadata.height;
        } else if (transaction.extraPayload) {
          transactionHeight = transaction.extraPayload.height;
        }

        // We check maturity is at least 100 blocks.
        // another way is to just read _scriptBuffer height value.
        if (transactionHeight + options.coinbaseMaturity > currentBlockHeight) {
          skipUtxo = true;
        }
      }

      if (!skipUtxo) {
        utxos.push(new Transaction.UnspentOutput(
          {
            txId: txid,
            vout: parseInt(outputIndex, 10),
            script: addressData.utxos[utxoIdentifier].script,
            satoshis: addressData.utxos[utxoIdentifier].satoshis,
            address: new Address(addressData.address, network),
          },
        ));
      }
    });
  });
  return utxos.sort((a, b) => b.satoshis - a.satoshis);
}

module.exports = getUTXOS;
