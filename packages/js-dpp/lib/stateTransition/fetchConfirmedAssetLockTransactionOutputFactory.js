const { Transaction } = require('@dashevo/dashcore-lib');
const WrongOutPointError = require('@dashevo/dashcore-lib/lib/errors/WrongOutPointError');

const IdentityAssetLockTransactionNotFoundError = require('../errors/IdentityAssetLockTransactionNotFoundError');
const IdentityAssetLockTransactionOutputNotFoundError = require('../errors/IdentityAssetLockTransactionOutputNotFoundError');
const InvalidIdentityAssetLockTransactionOutPointError = require('../errors/InvalidIdentityAssetLockTransactionOutPointError');
const IdentityAssetLockTransactionIsNotConfirmedError = require('../errors/IdentityAssetLockTransactionIsNotConfirmedError');

/**
 *
 * @param {StateRepository} stateRepository
 * @param {function} parseTransactionOutPointBuffer
 * @param {boolean} [enableAssetLockTxOneBlockConfirmationFallback]
 * @return {fetchConfirmedAssetLockTransactionOutput}
 */
function fetchConfirmedAssetLockTransactionOutputFactory(
  stateRepository,
  parseTransactionOutPointBuffer,
  enableAssetLockTxOneBlockConfirmationFallback = false,
) {
  /**
   * Returns lock transaction output for provided lockedOutPoint
   *
   * @typedef fetchConfirmedAssetLockTransactionOutput
   * @param {string} lockedOutPoint
   * @return {Promise<Object>}
   */
  async function fetchConfirmedAssetLockTransactionOutput(lockedOutPoint) {
    let transactionHash;
    let outputIndex;

    const lockedOutBuffer = Buffer.from(lockedOutPoint, 'base64');

    try {
      ({ transactionHash, outputIndex } = parseTransactionOutPointBuffer(lockedOutBuffer));
    } catch (e) {
      if (e instanceof WrongOutPointError) {
        throw new InvalidIdentityAssetLockTransactionOutPointError(e.message);
      } else {
        throw e;
      }
    }

    const rawTransaction = await stateRepository.fetchTransaction(transactionHash);

    if (!rawTransaction) {
      throw new IdentityAssetLockTransactionNotFoundError(transactionHash);
    }

    let txIsConfirmed = rawTransaction.chainlock || rawTransaction.instantlock;
    if (!txIsConfirmed && enableAssetLockTxOneBlockConfirmationFallback) {
      txIsConfirmed = rawTransaction.confirmations > 0;
    }

    if (!txIsConfirmed) {
      throw new IdentityAssetLockTransactionIsNotConfirmedError(transactionHash);
    }

    const transaction = new Transaction(rawTransaction.hex);

    if (!transaction.outputs[outputIndex]) {
      throw new IdentityAssetLockTransactionOutputNotFoundError(outputIndex);
    }

    return transaction.outputs[outputIndex];
  }

  return fetchConfirmedAssetLockTransactionOutput;
}

module.exports = fetchConfirmedAssetLockTransactionOutputFactory;
