const Identifier = require('@dashevo/dpp/lib/identifier/Identifier');
const { hash } = require('@dashevo/dpp/lib/util/hash');
const SimplifiedMNList = require('@dashevo/dashcore-lib/lib/deterministicmnlist/SimplifiedMNList');

/**
 *
 * @param {DashPlatformProtocol} transactionalDpp
 * @param {DriveStateRepository|CachedStateRepositoryDecorator} transactionalStateRepository
 * @param {DataContractStoreRepository} dataContractRepository
 * @param {SimplifiedMasternodeList} simplifiedMasternodeList
 * @param {Identifier} masternodeRewardSharesContractId
 * @param {handleNewMasternode} handleNewMasternode
 * @param {handleUpdatedPubKeyOperator} handleUpdatedPubKeyOperator
 * @param {splitDocumentsIntoChunks} splitDocumentsIntoChunks
 * @param {number} smlMaxListsLimit
 * @param {RpcClient} coreRpcClient
 * @return {synchronizeMasternodeIdentities}
 */
function synchronizeMasternodeIdentitiesFactory(
  transactionalDpp,
  transactionalStateRepository,
  dataContractRepository,
  simplifiedMasternodeList,
  masternodeRewardSharesContractId,
  handleNewMasternode,
  handleUpdatedPubKeyOperator,
  splitDocumentsIntoChunks,
  smlMaxListsLimit,
  coreRpcClient,
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

    const currentMNList = simplifiedMasternodeList.getStore()
      .getSMLbyHeight(coreHeight)
      .mnList;

    if (lastSyncedCoreHeight === 0) {
      // Create identities for all masternodes on the first sync
      newMasternodes = currentMNList;
    } else {
      // simplifiedMasternodeList contains sml only for the last `smlMaxListsLimit` number of blocks
      if (coreHeight - lastSyncedCoreHeight > smlMaxListsLimit) {
        // get diff directly from core
        const { result: rawDiff } = await coreRpcClient.protx('diff', 1, lastSyncedCoreHeight);

        previousMNList = new SimplifiedMNList(rawDiff).mnList;
      } else {
        previousMNList = simplifiedMasternodeList.getStore()
          .getSMLbyHeight(lastSyncedCoreHeight)
          .mnList;
      }

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

    const contract = await dataContractRepository.fetch(masternodeRewardSharesContractId, true);

    // Create identities and shares for new masternodes
    const documentsToModify = [];
    for (const newMasternodeEntry of newMasternodes) {
      const documents = await handleNewMasternode(newMasternodeEntry, contract);

      documentsToModify.push(documents);
    }

    // update operator identities (PubKeyOperator is changed)
    for (const mnEntry of masternodesWithNewPubKeyOperator) {
      const previousMnEntry = previousMNList.find((previousMnListEntry) => (
        previousMnListEntry.proRegTxHash === mnEntry.proRegTxHash
        && previousMnListEntry.pubKeyOperator !== mnEntry.pubKeyOperator
      ));

      const documents = handleUpdatedPubKeyOperator(
        mnEntry,
        previousMnEntry,
        contract,
      );

      documentsToModify.push(documents);
    }

    documentsToModify.forEach((item) => {
      documentsToCreate = documentsToCreate.concat(item.create);
      documentsToDelete = documentsToDelete.concat(item.delete);
    });

    lastSyncedCoreHeight = coreHeight;

    // Remove masternode reward shares for invalid/removed masternodes
    const disappearedOrInvalidMasterNodes = previousMNList
      .filter((previousMnListEntry) =>
        // eslint-disable-next-line max-len,implicit-arrow-linebreak
        (!currentMNList.find((currentMnListEntry) => currentMnListEntry.proRegTxHash === previousMnListEntry.proRegTxHash)))
      .concat(currentMNList.filter((currentMnListEntry) => !currentMnListEntry.isValid));

    for (const masternodeEntry of disappearedOrInvalidMasterNodes) {
      const doubleSha256Hash = hash(Buffer.from(masternodeEntry.proRegTxHash, 'hex'));

      //  Delete documents belongs to masternode identity (ownerId) from rewards contract
      const documents = await transactionalStateRepository.fetchDocuments(
        masternodeRewardSharesContractId,
        'masternodeRewardShares',
        {
          where: [
            ['$ownerId', '==', Identifier.from(doubleSha256Hash)],
          ],
        },
      );

      documentsToDelete = documentsToDelete.concat(documents);
    }

    // Process masternode reward contract updates
    if (documentsToCreate.length > 0 || documentsToDelete > 0) {
      const chunkedDocuments = splitDocumentsIntoChunks({
        create: documentsToCreate,
        delete: documentsToDelete,
      });

      for (const documentsChunk of chunkedDocuments) {
        const documentsBatchTransition = transactionalDpp.document.createStateTransition(
          documentsChunk,
        );

        await transactionalDpp.stateTransition.apply(documentsBatchTransition);
      }
    }
  }

  return synchronizeMasternodeIdentities;
}

module.exports = synchronizeMasternodeIdentitiesFactory;
