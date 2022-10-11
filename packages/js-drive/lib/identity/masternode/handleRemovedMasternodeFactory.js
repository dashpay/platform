/**
 *
 * @returns {handleRemovedMasternode}
 */
function handleRemovedMasternodeFactory(
  documentRepository,
) {
  /**
   * @typedef {handleRemovedMasternode}
   */
  async function handleRemovedMasternode(masternodeIdentifier, dataContract) {
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
        useTransaction: true,
      },
    );

    const documentsToDelete = fetchedDocumentsResult.getValue();

    for (const document of documentsToDelete) {
      await documentRepository.delete(
        dataContract,
        'rewardShare',
        document.getId(),
        { useTransaction: true },
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
