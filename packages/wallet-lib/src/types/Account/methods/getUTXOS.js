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
  const currentBlockHeight = chainStore.blockHeight;

  Object.values(accountState.addresses).forEach((address) => {
    const addressData = chainStore.getAddress(address);
    const utxosKeys = Object.keys(addressData.utxos);

    utxosKeys.forEach((utxoIdentifier) => {
      let skipUtxo = false;
      const [txid, outputIndex] = utxoIdentifier.split('-');
      const { transaction } = chainStore.getTransaction(txid);
      if (transaction.isCoinbase()) {
        // If the transaction is not a special transaction, we can't check its
        // maturity at the moment of writing this comment.
        // The wallet library doesn't maintain the header chain and thus we can
        // figure out the height only from the payload, but old coinbase transactions
        // doesn't have a payload.
        if (transaction.isSpecialTransaction()) {
          const transactionHeight = (this.store.transactionsMetadata[txid])
            ? this.store.transactionsMetadata[txid].height
            : transaction.extraPayload.height;

          // We check maturity is at least 100 blocks.
          // another way is to just read _scriptBuffer height value.
          if (transactionHeight + options.coinbaseMaturity > currentBlockHeight) {
            skipUtxo = true;
          }
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
