const { Transaction } = require('@dashevo/dashcore-lib');
const WrongOutPointError = require('@dashevo/dashcore-lib/lib/errors/WrongOutPointError');

const IdentityLockTransactionNotFoundError = require('../errors/IdentityLockTransactionNotFoundError');
const InvalidIdentityOutPointError = require('../errors/InvalidIdentityOutPointError');

/**
 *
 * @param {StateRepository} stateRepository
 * @param {function} parseTransactionOutPointBuffer
 * @return {getLockedTransactionOutput}
 */
function getLockedTransactionOutputFactory(
  stateRepository,
  parseTransactionOutPointBuffer,
) {
  /**
   * Returns lock transaction output for provided lockedOutPoint
   *
   * @typedef getLockedTransactionOutput
   * @param {string} lockedOutPoint
   * @return {Promise<Object>}
   */
  async function getLockedTransactionOutput(lockedOutPoint) {
    let transactionHash;
    let outputIndex;

    const lockedOutBuffer = Buffer.from(lockedOutPoint, 'base64');

    try {
      ({ transactionHash, outputIndex } = parseTransactionOutPointBuffer(lockedOutBuffer));
    } catch (e) {
      if (e instanceof WrongOutPointError) {
        throw new InvalidIdentityOutPointError(e.message);
      } else {
        throw e;
      }
    }

    const rawTransaction = await stateRepository.fetchTransaction(transactionHash);

    if (!rawTransaction) {
      throw new IdentityLockTransactionNotFoundError(transactionHash);
    }

    const transaction = new Transaction(rawTransaction);

    if (!transaction.outputs[outputIndex]) {
      throw new InvalidIdentityOutPointError(`Output with index ${outputIndex} not found`);
    }

    return transaction.outputs[outputIndex];
  }

  return getLockedTransactionOutput;
}

module.exports = getLockedTransactionOutputFactory;
