/**
 * @param {Identifier} featureFlagDataContractId
 * @param {Long} featureFlagDataContractBlockHeight
 * @param {fetchDocuments} fetchDocuments
 *
 * @return {getFeatureFlagForHeight}
 */
function getFeatureFlagForHeightFactory(
  featureFlagDataContractId,
  featureFlagDataContractBlockHeight,
  fetchDocuments,
) {
  /**
   * @typedef getFeatureFlagForHeight
   *
   * @param {string} flagType
   * @param {Long} blockHeight
   * @param {GroveDBTransaction} [transaction]
   *
   * @return {Promise<Document|null>}
   */
  async function getFeatureFlagForHeight(flagType, blockHeight, transaction = undefined) {
    if (!featureFlagDataContractId) {
      return null;
    }

    if (blockHeight.lte(featureFlagDataContractBlockHeight)) {
      return null;
    }

    const query = {
      where: [
        ['enableAtHeight', '==', blockHeight.toNumber()],
      ],
    };

    const [document] = await fetchDocuments(
      featureFlagDataContractId,
      flagType,
      query,
      transaction,
    );

    return document;
  }

  return getFeatureFlagForHeight;
}

module.exports = getFeatureFlagForHeightFactory;
