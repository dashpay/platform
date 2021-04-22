/**
 * @param {Identifier} featureFlagDataContractId
 * @param {fetchDocuments} fetchDocuments
 *
 * @return {getFeatureFlagForHeight}
 */
function getFeatureFlagForHeightFactory(
  featureFlagDataContractId,
  fetchDocuments,
) {
  /**
   * @typedef getFeatureFlagForHeight
   *
   * @param {string} flagType
   * @param {Long} blockHeight
   *
   * @return {Promise<Document|null>}
   */
  async function getFeatureFlagForHeight(flagType, blockHeight) {
    if (!featureFlagDataContractId) {
      return null;
    }

    const query = {
      where: [
        ['enableAtHeight', '==', blockHeight.toInt()],
      ],
    };

    const [document] = await fetchDocuments(
      featureFlagDataContractId,
      flagType,
      query,
    );

    return document;
  }

  return getFeatureFlagForHeight;
}

module.exports = getFeatureFlagForHeightFactory;
