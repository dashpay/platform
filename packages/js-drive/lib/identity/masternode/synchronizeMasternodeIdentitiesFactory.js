const Identifier = require('@dashevo/dpp/lib/identifier/Identifier');
const { hash } = require('@dashevo/dpp/lib/util/hash');

/**
 *
 * @param {DashPlatformProtocol} transactionalDpp
 * @param {DriveStateRepository|CachedStateRepositoryDecorator} stateRepository
 * @param {SimplifiedMasternodeList} simplifiedMasternodeList
 * @param {Identifier} masternodeRewardSharesContractId
 * @param {handleNewMasternode} handleNewMasternode
 * @param {handleUpdatedPubKeyOperator} handleUpdatedPubKeyOperator
 * @param {splitDocumentsIntoChunks} splitDocumentsIntoChunks
 * @return {synchronizeMasternodeIdentities}
 */
function synchronizeMasternodeIdentitiesFactory(
  transactionalDpp,
  stateRepository,
  simplifiedMasternodeList,
  masternodeRewardSharesContractId,
  handleNewMasternode,
  handleUpdatedPubKeyOperator,
  splitDocumentsIntoChunks, // TODO - DI!!!!
) {
  let lastSyncedCoreHeight = 0;

  /**
   * @typedef synchronizeMasternodeIdentities
   * @param {number} coreHeight
   * @return Promise<void>
   */
  async function synchronizeMasternodeIdentities(coreHeight) {
    let documentsToCreate = [];
    let documentsToDelete = [];

    let newMasternodes = [];
    let masternodesWithNewPubKeyOperator = [];

    let previousMNList = [];

    if (lastSyncedCoreHeight > 0) {
      previousMNList = simplifiedMasternodeList.getStore()
        .getSMLbyHeight(lastSyncedCoreHeight)
        .mnList;
    }

    const currentMNList = simplifiedMasternodeList.getStore()
      .getSMLbyHeight(coreHeight)
      .mnList;

    if (lastSyncedCoreHeight === 0) {
      // Create identities for all masternodes on the first sync
      newMasternodes = simplifiedMasternodeList.getStore().getCurrentSML().mnList;
    } else {
      // Get the difference between last sync and requested core height
      newMasternodes = currentMNList.filter((currentMnListEntry) => (
        !previousMNList.find((previousMnListEntry) => (
          previousMnListEntry.proRegTxHash === currentMnListEntry.proRegTxHash
        ))
      ));

      masternodesWithNewPubKeyOperator = currentMNList.filter((currentMnListEntry) => (
        previousMNList.find((previousMnListEntry) => (
          previousMnListEntry.proRegTxHash === currentMnListEntry.proRegTxHash
          && previousMnListEntry.pubKeyOperator !== currentMnListEntry.pubKeyOperator
        ))
      ));
    }

    // Create identities and shares for new masternodes
    let documentsToModify = await Promise.all(
      newMasternodes.map(handleNewMasternode),
    );

    documentsToModify = documentsToModify.concat(await Promise.all(
      masternodesWithNewPubKeyOperator.map(async (mnEntry) => {
        const previousMnEntry = previousMNList.find((previousMnListEntry) => (
          previousMnListEntry.proRegTxHash === mnEntry.proRegTxHash
          && previousMnListEntry.pubKeyOperator !== mnEntry.pubKeyOperator
        ));

        return handleUpdatedPubKeyOperator(mnEntry, previousMnEntry);
      }),
    ));

    documentsToModify.forEach((item) => {
      documentsToCreate = documentsToCreate.concat(item.create);
      documentsToDelete = documentsToDelete.concat(item.delete);
    });

    lastSyncedCoreHeight = coreHeight;

    // PubKeyOperator is changed

    // A masternode disappeared or is not valid
    const disappearedOrInvalidMasterNodes = previousMNList
      .filter((previousMnListEntry) =>
        // eslint-disable-next-line max-len,implicit-arrow-linebreak
        (!currentMNList.find((currentMnListEntry) => currentMnListEntry.proRegTxHash === previousMnListEntry.proRegTxHash)))
      .concat(currentMNList.filter((currentMnListEntry) => !currentMnListEntry.isValid));

    await Promise.all(
      disappearedOrInvalidMasterNodes.map(async (masternodeEntry) => {
        const doubleSha256Hash = hash(Buffer.from(masternodeEntry.proRegTxHash, 'hex'));
        //  Delete documents belongs to masternode identity (ownerId) from rewards contract
        const documents = await stateRepository.fetchDocuments(
          masternodeRewardSharesContractId,
          'masternodeRewardShares',
          {
            where: [
              ['$ownerId', '===', Identifier.from(doubleSha256Hash),
              ],
            ],
          },
        );

        documentsToDelete = documentsToDelete.concat(documents);
      }),
    );

    if (documentsToCreate.length > 0 || documentsToDelete > 0) {
      const chunkedDocuments = splitDocumentsIntoChunks({
        create: documentsToCreate,
        delete: documentsToDelete,
      });

      await Promise.all(
        chunkedDocuments.map(async (documentsChunk) => {
          const documentsBatchTransition = transactionalDpp.document.createStateTransition(
            documentsChunk,
          );

          await transactionalDpp.stateTransition.apply(documentsBatchTransition);
        }),
      );
    }
  }

  return synchronizeMasternodeIdentities;
}

module.exports = synchronizeMasternodeIdentitiesFactory;
