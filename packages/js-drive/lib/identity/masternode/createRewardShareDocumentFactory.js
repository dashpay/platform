const { hash } = require('@dashevo/dpp/lib/util/hash');
const Identifier = require('@dashevo/dpp/lib/identifier/Identifier');

const MAX_DOCUMENTS = 16;

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
        useTransaction: true,
      },
    );

    // Reward share for this operator is already exists
    if (!documentsResult.isEmpty()) {
      return false;
    }

    const fetchedDocumentsResult = await documentRepository.find(
      dataContract,
      'rewardShare',
      {
        where: [
          ['$ownerId', '==', masternodeIdentifier],
        ],
        useTransaction: true,
      },
    );

    if (
      !fetchedDocumentsResult.isEmpty()
      && fetchedDocumentsResult.getValue().length > MAX_DOCUMENTS
    ) {
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

    await documentRepository.create(rewardShareDocument, {
      useTransaction: true,
    });

    return true;
  }

  return createRewardShareDocument;
}

module.exports = createRewardShareDocumentFactory;
