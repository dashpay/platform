const AssetLockOutputNotFoundError = require('../../errors/AssetLockOutputNotFoundError');

/**
 * @param {fetchAssetLockTransactionOutput} fetchAssetLockTransactionOutput
 * @return {fetchAssetLockPublicKeyHash}
 */
function fetchAssetLockPublicKeyHashFactory(fetchAssetLockTransactionOutput) {
  /**
   * @typedef {fetchAssetLockPublicKeyHash}
   * @param {InstantAssetLockProof|ChainAssetLockProof} assetLockProof
   * @param {StateTransitionExecutionContext} executionContext
   * @return {Promise<Buffer>}
   */
  async function fetchAssetLockPublicKeyHash(assetLockProof, executionContext) {
    const output = await fetchAssetLockTransactionOutput(assetLockProof, executionContext);

    if (!output) {
      throw new AssetLockOutputNotFoundError();
    }

    return output.script.getData();
  }

  return fetchAssetLockPublicKeyHash;
}

module.exports = fetchAssetLockPublicKeyHashFactory;
