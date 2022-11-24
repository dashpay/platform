/**
 * @param {Identifier} featureFlagsContractId
 * @param {fetchDocuments} fetchDocuments
 *
 * @return {getLatestFeatureFlag}
 */
function getLatestFeatureFlagFactory(
  featureFlagsContractId,
  fetchDocuments,
) {
  /**
   * @typedef getLatestFeatureFlag
   *
   * @param {string} flagType
   * @param {Long} blockHeight
   * @param {GroveDBTransaction} transaction
   *
   * @return {Promise<Document|null>}
   */
  async function getLatestFeatureFlag(flagType, blockHeight, transaction = undefined) {
    if (!featureFlagsContractId) {
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

    const result = await fetchDocuments(
      featureFlagsContractId,
      flagType,
      {
        ...query,
        transaction,
      },
    );

    const [document] = result.getValue();

    return document;
  }

  return getLatestFeatureFlag;
}

module.exports = getLatestFeatureFlagFactory;
