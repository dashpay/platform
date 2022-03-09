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
   * @param {Identifier} masternodeIdentityId
   * @param {Identifier} operatorIdentityId
   * @param {number} percentage
   * @returns {Promise<boolean>}
   */
  async function createRewardShareDocument(
    dataContract,
    masternodeIdentityId,
    operatorIdentityId,
    percentage,
  ) {
    const documents = await documentRepository.find(
      dataContract,
      'rewardShare',
      {
        where: [
          ['$ownerId', '==', masternodeIdentityId.toBuffer()],
          ['payToId', '==', operatorIdentityId.toBuffer()],
        ],
      },
      true,
    );

    // Reward share for this operator is already exists
    if (documents.length > 0) {
      return false;
    }

    const rewardShareDocument = dpp.document.create(
      dataContract,
      masternodeIdentityId,
      'rewardShare',
      {
        payToId: operatorIdentityId,
        percentage,
      },
    );

    // Create an identity for operator
    const rewardShareDocumentIdSeed = hash(
      Buffer.concat([
        masternodeIdentityId.toBuffer(),
        operatorIdentityId.toBuffer(),
      ]),
    );

    rewardShareDocument.id = Identifier.from(rewardShareDocumentIdSeed);

    await documentRepository.store(rewardShareDocument, true);

    return true;
  }

  return createRewardShareDocument;
}

module.exports = createRewardShareDocumentFactory;
