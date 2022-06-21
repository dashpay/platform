/**
 * @param {Identifier} featureFlagsContractId
 * @param {fetchDocuments} fetchDocuments
 * @param {fetchDocuments} fetchDataContract
 *
 * @return {getFeatureFlagForHeight}
 */
function getFeatureFlagForHeightFactory(
  featureFlagsContractId,
  fetchDocuments,
  fetchDataContract,
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

    const dataContractResult = await fetchDataContract(featureFlagsContractId, flagType);

    const result = await fetchDocuments(
      dataContractResult,
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
