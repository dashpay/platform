const Identifier = require('@dashevo/dpp/lib/identifier/Identifier');
const SimplifiedMNList = require('@dashevo/dashcore-lib/lib/deterministicmnlist/SimplifiedMNList');
const createOperatorIdentifier = require('./createOperatorIdentifier');

/**
 *
 * @param {DataContractStoreRepository} dataContractRepository
 * @param {SimplifiedMasternodeList} simplifiedMasternodeList
 * @param {Identifier} masternodeRewardSharesContractId
 * @param {handleNewMasternode} handleNewMasternode
 * @param {handleUpdatedPubKeyOperator} handleUpdatedPubKeyOperator
 * @param {handleRemovedMasternode} handleRemovedMasternode
 * @param {handleUpdatedScriptPayout} handleUpdatedScriptPayout
 * @param {number} smlMaxListsLimit
 * @param {RpcClient} coreRpcClient
 * @return {synchronizeMasternodeIdentities}
 */
function synchronizeMasternodeIdentitiesFactory(
  dataContractRepository,
  simplifiedMasternodeList,
  masternodeRewardSharesContractId,
  handleNewMasternode,
  handleUpdatedPubKeyOperator,
  handleRemovedMasternode,
  handleUpdatedScriptPayout,
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
    let newMasternodes = [];

    let previousMNList = [];

    const currentMNList = simplifiedMasternodeList.getStore()
      .getSMLbyHeight(coreHeight)
      .mnList;

    const dataContract = await dataContractRepository.fetch(masternodeRewardSharesContractId, true);

    if (lastSyncedCoreHeight === 0) {
      // Create identities for all masternodes on the first sync
      newMasternodes = currentMNList;
    } else {
      // simplifiedMasternodeList contains sml only for the last `smlMaxListsLimit` number of blocks
      if (coreHeight - lastSyncedCoreHeight >= smlMaxListsLimit) {
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

      // Update operator identities (PubKeyOperator is changed)
      for (const mnEntry of currentMNList) {
        const previousMnEntry = previousMNList.find((previousMnListEntry) => (
          previousMnListEntry.proRegTxHash === mnEntry.proRegTxHash
          && previousMnListEntry.pubKeyOperator !== mnEntry.pubKeyOperator
        ));

        if (previousMnEntry) {
          await handleUpdatedPubKeyOperator(
            mnEntry,
            previousMnEntry,
            dataContract,
          );
        }

        const scriptPayoutIsChanged = previousMNList.find((previousMnListEntry) => (
          previousMnListEntry.proRegTxHash === mnEntry.proRegTxHash
          && previousMnListEntry.scriptPayout !== mnEntry.scriptPayout
        ));

        if (scriptPayoutIsChanged) {
          await handleUpdatedScriptPayout(
            Identifier.from(Buffer.from(Identifier.proRegTxHash, 'hex')),
            mnEntry.scriptPayout,
            previousMnEntry.scriptPayout,
          );
        }

        const scriptOperatorPayoutIsChanged = previousMNList.find((previousMnListEntry) => (
          previousMnListEntry.proRegTxHash === mnEntry.proRegTxHash
          && previousMnListEntry.scriptOperatorPayout !== mnEntry.scriptOperatorPayout
        ));

        if (scriptOperatorPayoutIsChanged) {
          await handleUpdatedScriptPayout(
            createOperatorIdentifier(mnEntry),
            mnEntry.scriptOperatorPayout,
            previousMnEntry.scriptOperatorPayout,
          );
        }
      }
    }

    // Create identities and shares for new masternodes

    for (const newMasternodeEntry of newMasternodes) {
      await handleNewMasternode(newMasternodeEntry, dataContract);
    }

    // Remove masternode reward shares for invalid/removed masternodes

    const disappearedOrInvalidMasterNodes = previousMNList
      .filter((previousMnListEntry) =>
        // eslint-disable-next-line max-len,implicit-arrow-linebreak
        (!currentMNList.find((currentMnListEntry) => currentMnListEntry.proRegTxHash === previousMnListEntry.proRegTxHash)))
      .concat(currentMNList.filter((currentMnListEntry) => !currentMnListEntry.isValid));

    for (const masternodeEntry of disappearedOrInvalidMasterNodes) {
      const masternodeIdentifier = Identifier.from(
        Buffer.from(masternodeEntry.proRegTxHash, 'hex'),
      );

      await handleRemovedMasternode(
        masternodeIdentifier,
        dataContract,
      );
    }

    lastSyncedCoreHeight = coreHeight;
  }

  return synchronizeMasternodeIdentities;
}

module.exports = synchronizeMasternodeIdentitiesFactory;
