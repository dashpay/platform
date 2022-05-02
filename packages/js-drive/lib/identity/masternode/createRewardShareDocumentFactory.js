const { hash } = require('@dashevo/dpp/lib/util/hash');
const Identifier = require('@dashevo/dpp/lib/identifier/Identifier');

/**
 * @param {DashPlatformProtocol} dpp
 * @param {DocumentRepository} documentRepository
 * @return {createRewardShareDocument}
 */
function createRewardShareDocumentFactory(
  dpp,
  documentRepository,
) {
  /**
   * @typedef {createRewardShareDocument}
   * @param {DataContract} dataContract
   * @param {Identifier} masternodeIdentifier
   * @param {Identifier} operatorIdentifier
   * @param {number} percentage
   * @returns {Promise<boolean>}
   */
  async function createRewardShareDocument(
    dataContract,
    masternodeIdentifier,
    operatorIdentifier,
    percentage,
  ) {
    const documentsResult = await documentRepository.find(
      dataContract,
      'rewardShare',
      {
        where: [
          ['$ownerId', '==', masternodeIdentifier.toBuffer()],
          ['payToId', '==', operatorIdentifier.toBuffer()],
        ],
      },
      true,
    );

    // Reward share for this operator is already exists
    if (!documentsResult.isEmpty()) {
      return false;
    }

    const rewardShareDocument = dpp.document.create(
      dataContract,
      masternodeIdentifier,
      'rewardShare',
      {
        payToId: operatorIdentifier,
        percentage,
      },
    );

    // Create an identity for operator
    const rewardShareDocumentIdSeed = hash(
      Buffer.concat([
        masternodeIdentifier.toBuffer(),
        operatorIdentifier.toBuffer(),
      ]),
    );

    rewardShareDocument.id = Identifier.from(rewardShareDocumentIdSeed);

    await documentRepository.store(rewardShareDocument, true);

    return true;
  }

  return createRewardShareDocument;
}

module.exports = createRewardShareDocumentFactory;
