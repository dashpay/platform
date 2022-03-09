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
  async function handleRemovedMasternode(masternodeOwnerId, dataContract) {
    //  Delete documents belongs to masternode identity (ownerId) from rewards contract

    let documentsToDelete = [];

    let startAfter;
    let fetchedDocuments;
    const limit = 100;

    do {
      fetchedDocuments = await documentRepository.find(
        dataContract,
        'rewardShare',
        {
          limit,
          startAfter,
          where: [
            ['$ownerId', '==', masternodeOwnerId],
          ],
        },
        true,
      );

      documentsToDelete = documentsToDelete.concat(fetchedDocuments);

      startAfter = fetchedDocuments.length > 0
        ? fetchedDocuments[fetchedDocuments.length - 1].id : undefined;
    } while (fetchedDocuments.length === limit);

    for (const document of documentsToDelete) {
      await documentRepository.delete(
        dataContract,
        'rewardShare',
        document.getId(),
        true,
      );
    }
  }

  return handleRemovedMasternode;
}

module.exports = handleRemovedMasternodeFactory;
