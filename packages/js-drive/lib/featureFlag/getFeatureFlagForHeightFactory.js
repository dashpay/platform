/**
 * @param {Identifier} featureFlagsContractId
 * @param {fetchDocuments} fetchDocuments
 *
 * @return {getFeatureFlagForHeight}
 */
function getFeatureFlagForHeightFactory(
  featureFlagsContractId,
  fetchDocuments,
) {
  /**
   * @typedef getFeatureFlagForHeight
   *
   * @param {string} flagType
   * @param {Long} blockHeight
   * @param {boolean} [useTransaction=false]
   *
   * @return {Promise<Document|null>}
   */
  async function getFeatureFlagForHeight(flagType, blockHeight, useTransaction = false) {
    if (!featureFlagsContractId) {
      return null;
    }

    const query = {
      where: [
        ['enableAtHeight', '==', blockHeight.toNumber()],
      ],
    };

    const result = await fetchDocuments(
      featureFlagsContractId,
      flagType,
      {
        ...query,
        useTransaction,
      },
    );

    const [document] = result.getValue();

    return document;
  }

  return getFeatureFlagForHeight;
}

module.exports = getFeatureFlagForHeightFactory;
