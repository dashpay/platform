const AssetLockOutputNotFoundError = require('../../errors/AssetLockOutputNotFoundError');

/**
 * @param {fetchAssetLockTransactionOutput} fetchAssetLockTransactionOutput
 * @return {fetchAssetLockPublicKeyHash}
 */
function fetchAssetLockPublicKeyHashFactory(fetchAssetLockTransactionOutput) {
  /**
   * @typedef {fetchAssetLockPublicKeyHash}
   * @param {InstantAssetLockProof|ChainAssetLockProof} assetLockProof
   * @return {Promise<Buffer>}
   */
  async function fetchAssetLockPublicKeyHash(assetLockProof) {
    const output = await fetchAssetLockTransactionOutput(assetLockProof);

    if (!output) {
      throw new AssetLockOutputNotFoundError();
    }

    return output.script.getData();
  }

  return fetchAssetLockPublicKeyHash;
}

module.exports = fetchAssetLockPublicKeyHashFactory;
