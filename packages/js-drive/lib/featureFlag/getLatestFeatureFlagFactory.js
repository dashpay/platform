/**
 * @param {Identifier} featureFlagDataContractId
 * @param {Long} featureFlagDataContractBlockHeight
 * @param {fetchDocuments} fetchDocuments
 *
 * @return {getLatestFeatureFlag}
 */
function getLatestFeatureFlagFactory(
  featureFlagDataContractId,
  featureFlagDataContractBlockHeight,
  fetchDocuments,
) {
  /**
   * @typedef getLatestFeatureFlag
   *
   * @param {string} flagType
   * @param {Long} blockHeight
   * @param {DocumentsIndexedTransaction} [transaction]
   *
   * @return {Promise<Document|null>}
   */
  async function getLatestFeatureFlag(flagType, blockHeight, transaction = undefined) {
    if (!featureFlagDataContractId) {
      return null;
    }

    if (blockHeight.lt(featureFlagDataContractBlockHeight)) {
      return null;
    }

    const query = {
      where: [
        ['enableAtHeight', '<=', blockHeight.toNumber()],
      ],
      orderBy: [
        ['enableAtHeight', 'desc'],
      ],
      limit: 1,
    };

    const [document] = await fetchDocuments(
      featureFlagDataContractId,
      flagType,
      query,
      transaction,
    );

    return document;
  }

  return getLatestFeatureFlag;
}

module.exports = getLatestFeatureFlagFactory;
