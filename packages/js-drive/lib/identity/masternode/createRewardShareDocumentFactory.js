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
   * @param {BlockInfo} blockInfo
   * @returns {Promise<Document|null>}
   */
  async function createRewardShareDocument(
    dataContract,
    masternodeIdentifier,
    operatorIdentifier,
    percentage,
    blockInfo,
  ) {
    const documentsResult = await documentRepository.find(
      dataContract,
      'rewardShare',
      blockInfo,
      {
        where: [
          ['$ownerId', '==', masternodeIdentifier.toBuffer()],
        ],
        useTransaction: true,
      },
    );

    // Do not create a share if it's exist already
    // or max shares limit is reached
    if (!documentsResult.isEmpty()) {
      if (documentsResult.getValue().length > MAX_DOCUMENTS) {
        return null;
      }

      const operatorShare = documentsResult.getValue().find((shareDocument) => (
        shareDocument.get('payToId').equals(operatorIdentifier)
      ));

      if (operatorShare) {
        return null;
      }
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

    await documentRepository.create(rewardShareDocument, blockInfo, {
      useTransaction: true,
    });

    return rewardShareDocument;
  }

  return createRewardShareDocument;
}

module.exports = createRewardShareDocumentFactory;
