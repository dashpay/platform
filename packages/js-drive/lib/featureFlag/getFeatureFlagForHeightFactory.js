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
   * @param {GroveDBTransaction} [transaction]
   *
   * @return {Promise<Document|null>}
   */
  async function getFeatureFlagForHeight(flagType, blockHeight, transaction = undefined) {
    if (!featureFlagsContractId) {
      return null;
    }

    const query = {
      where: [
        ['enableAtHeight', '==', blockHeight.toNumber()],
      ],
    };

    const [document] = await fetchDocuments(
      featureFlagsContractId,
      flagType,
      query,
      transaction,
    );

    return document;
  }

  return getFeatureFlagForHeight;
}

module.exports = getFeatureFlagForHeightFactory;
