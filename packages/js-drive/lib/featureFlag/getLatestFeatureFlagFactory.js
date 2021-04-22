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
   *
   * @return {Promise<Document|null>}
   */
  async function getLatestFeatureFlag(flagType, blockHeight) {
    if (!featureFlagDataContractId) {
      return null;
    }

    if (blockHeight.lt(featureFlagDataContractBlockHeight)) {
      return null;
    }

    const query = {
      where: [
        ['enableAtHeight', '<=', blockHeight.toInt()],
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
    );

    return document;
  }

  return getLatestFeatureFlag;
}

module.exports = getLatestFeatureFlagFactory;
