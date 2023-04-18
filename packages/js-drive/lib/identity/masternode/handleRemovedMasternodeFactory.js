/**
 * @param {DocumentRepository} documentRepository
 *
 * @returns {handleRemovedMasternode}
 */
function handleRemovedMasternodeFactory(
  documentRepository,
) {
  /**
   * @typedef {handleRemovedMasternode}
   *
   * @param {Identifier} masternodeIdentifier
   * @param {DataContract} dataContract
   * @param {BlockInfo} blockInfo
   * @return {Promise<{
   *  createdEntities: Array<Identity|Document>,
   *  updatedEntities: Array<Identity>,
   *  removedEntities: Array<Document>,
   * }>}
   */
  async function handleRemovedMasternode(masternodeIdentifier, dataContract, blockInfo) {
    //  Delete documents belongs to masternode identity (ownerId) from rewards contract
    // since max amount is 16, we can fetch all of them in one request
    const removedEntities = [];

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

    const documentsToDelete = fetchedDocumentsResult.getValue();

    for (const document of documentsToDelete) {
      await documentRepository.delete(
        dataContract,
        'rewardShare',
        document.getId(),
        blockInfo,
        { useTransaction: true },
      );

      removedEntities.push(
        document,
      );
    }

    return {
      createdEntities: [],
      updatedEntities: [],
      removedEntities,
    };
  }

  return handleRemovedMasternode;
}

module.exports = handleRemovedMasternodeFactory;
