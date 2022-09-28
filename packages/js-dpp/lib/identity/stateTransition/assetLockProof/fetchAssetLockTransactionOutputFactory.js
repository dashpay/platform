const { Transaction, Script } = require('@dashevo/dashcore-lib');
const Output = require('@dashevo/dashcore-lib/lib/transaction/output');
const InstantAssetLockProof = require('./instant/InstantAssetLockProof');
const ChainAssetLockProof = require('./chain/ChainAssetLockProof');
const AssetLockTransactionIsNotFoundError = require('../../errors/AssetLockTransactionIsNotFoundError');
const UnknownAssetLockProofTypeError = require('../../errors/UnknownAssetLockProofTypeError');

/**
 * @param {StateRepository} stateRepository
 *
 * @returns {fetchAssetLockTransactionOutput}
 */

function fetchAssetLockTransactionOutputFactory(
  stateRepository,
) {
  /**
   *
   * @typedef fetchAssetLockTransactionOutput
   * @param {InstantAssetLockProof|ChainAssetLockProof} assetLockProof
   * @param {StateTransitionExecutionContext} executionContext
   * @returns {Promise<Output>}
   */
  async function fetchAssetLockTransactionOutput(assetLockProof, executionContext) {
    if (assetLockProof.getType() === InstantAssetLockProof.type) {
      return assetLockProof.getOutput();
    }

    if (assetLockProof.getType() === ChainAssetLockProof.type) {
      const outPoint = Transaction.parseOutPointBuffer(assetLockProof.getOutPoint());

      const { outputIndex, transactionHash } = outPoint;

      const rawTransaction = await stateRepository.fetchTransaction(
        transactionHash,
        executionContext,
      );

      if (executionContext.isDryRun()) {
        return new Output({
          satoshis: 1000,
          script: new Script(),
        });
      }

      if (rawTransaction === null) {
        throw new AssetLockTransactionIsNotFoundError(transactionHash);
      }

      const transaction = new Transaction(rawTransaction.data);
      return transaction.outputs[outputIndex];
    }

    throw new UnknownAssetLockProofTypeError(assetLockProof.getType());
  }

  return fetchAssetLockTransactionOutput;
}

module.exports = fetchAssetLockTransactionOutputFactory;
