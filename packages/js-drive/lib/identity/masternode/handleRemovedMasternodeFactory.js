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
   * @param {GroveDBTransaction} transaction
   */
  async function handleRemovedMasternode(masternodeIdentifier, dataContract, blockInfo, transaction) {
    //  Delete documents belongs to masternode identity (ownerId) from rewards contract
    // since max amount is 16, we can fetch all of them in one request
    const result = [];

    const fetchedDocumentsResult = await documentRepository.find(
      dataContract,
      'rewardShare',
      {
        where: [
          ['$ownerId', '==', masternodeIdentifier],
        ],
        transaction,
      },
    );

    const documentsToDelete = fetchedDocumentsResult.getValue();

    for (const document of documentsToDelete) {
      await documentRepository.delete(
        dataContract,
        'rewardShare',
        document.getId(),
        blockInfo,
        { transaction },
      );

      result.push(
        document,
      );
    }

    return result;
  }

  return handleRemovedMasternode;
}

module.exports = handleRemovedMasternodeFactory;
