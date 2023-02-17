const Identifier = require('@dashevo/dpp/lib/identifier/Identifier');
const Address = require('@dashevo/dashcore-lib/lib/address');
const Script = require('@dashevo/dashcore-lib/lib/script');
const createOperatorIdentifier = require('./createOperatorIdentifier');

/**
 *
 * @param result {{
 *  createdEntities: Array<Identity|Document>,
 *  updatedEntities: Array<Identity>,
 *  removedEntities: Array<Document>,
 *  }}
 * @param newData {{
 *  createdEntities: Array<Identity|Document>,
 *  updatedEntities: Array<Identity>,
 *  removedEntities: Array<Document>,
 *  }}
 * @return {{
 *  createdEntities: Array<Identity|Document>,
 *  updatedEntities: Array<Identity>,
 *  removedEntities: Array<Document>,
 *  }}
 */
function mergeEntities(result, newData) {
  return {
    ...result,
    createdEntities: result.createdEntities.concat(newData.createdEntities),
    updatedEntities: result.updatedEntities.concat(newData.updatedEntities),
    removedEntities: result.removedEntities.concat(newData.removedEntities),
  };
}

/**
 *
 * @param {DataContractStoreRepository} dataContractRepository
 * @param {SimplifiedMasternodeList} simplifiedMasternodeList
 * @param {Identifier} masternodeRewardSharesContractId
 * @param {handleNewMasternode} handleNewMasternode
 * @param {handleUpdatedPubKeyOperator} handleUpdatedPubKeyOperator
 * @param {handleRemovedMasternode} handleRemovedMasternode
 * @param {handleUpdatedScriptPayout} handleUpdatedScriptPayout
 * @param {handleUpdatedVotingAddress} handleUpdatedVotingAddress
 * @param {number} smlMaxListsLimit
 * @param {LastSyncedCoreHeightRepository} lastSyncedCoreHeightRepository
 * @param {fetchSimplifiedMNList} fetchSimplifiedMNList
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
  handleUpdatedVotingAddress,
  smlMaxListsLimit,
  lastSyncedCoreHeightRepository,
  fetchSimplifiedMNList,
) {
  let lastSyncedCoreHeight = 0;

  /**
   * @typedef synchronizeMasternodeIdentities
   * @param {number} coreHeight
   * @param {BlockInfo} blockInfo
   * @return {Promise<{
   *  createdEntities: Array<Identity|Document>,
   *  updatedEntities: Array<Identity>,
   *  removedEntities: Array<Document>,
   *  fromHeight: number,
   *  toHeight: number,
   * }>}
   */
  async function synchronizeMasternodeIdentities(coreHeight, blockInfo) {
    let result = {
      createdEntities: [],
      updatedEntities: [],
      removedEntities: [],
    };

    if (!lastSyncedCoreHeight) {
      const lastSyncedHeightResult = await lastSyncedCoreHeightRepository.fetch({
        useTransaction: true,
      });

      lastSyncedCoreHeight = lastSyncedHeightResult.getValue() || 0;
    }

    let newMasternodes;
    let previousMNList = [];

    const currentMNList = simplifiedMasternodeList.getStore()
      .getSMLbyHeight(coreHeight)
      .mnList;

    const dataContractResult = await dataContractRepository.fetch(
      masternodeRewardSharesContractId,
      {
        useTransaction: true,
      },
    );

    const dataContract = dataContractResult.getValue();

    if (lastSyncedCoreHeight === 0) {
      // Create identities for all masternodes on the first sync
      newMasternodes = currentMNList;
    } else {
      // simplifiedMasternodeList contains sml only for the last `smlMaxListsLimit` number of blocks
      if (coreHeight - lastSyncedCoreHeight >= smlMaxListsLimit) {
        // get diff directly from core
        ({ mnList: previousMNList } = await fetchSimplifiedMNList(1, lastSyncedCoreHeight));
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
          const affectedEntities = await handleUpdatedPubKeyOperator(
            mnEntry,
            previousMnEntry,
            dataContract,
            blockInfo,
          );

          result = mergeEntities(result, affectedEntities);
        }

        const previousVotingMnEntry = previousMNList.find((previousMnListEntry) => (
          previousMnListEntry.proRegTxHash === mnEntry.proRegTxHash
          && previousMnListEntry.votingAddress !== mnEntry.votingAddress
        ));

        if (previousVotingMnEntry) {
          const affectedEntities = await handleUpdatedVotingAddress(
            mnEntry,
            blockInfo,
          );

          result = mergeEntities(result, affectedEntities);
        }

        if (mnEntry.payoutAddress) {
          const mnEntryWithChangedPayoutAddress = previousMNList.find((previousMnListEntry) => (
            previousMnListEntry.proRegTxHash === mnEntry.proRegTxHash
            && previousMnListEntry.payoutAddress !== mnEntry.payoutAddress
          ));

          if (mnEntryWithChangedPayoutAddress) {
            const newPayoutScript = new Script(Address.fromString(mnEntry.payoutAddress));
            const previousPayoutScript = mnEntryWithChangedPayoutAddress.payoutAddress
              ? new Script(Address.fromString(mnEntryWithChangedPayoutAddress.payoutAddress))
              : undefined;

            const affectedEntities = await handleUpdatedScriptPayout(
              Identifier.from(Buffer.from(mnEntry.proRegTxHash, 'hex')),
              newPayoutScript,
              blockInfo,
              previousPayoutScript,
            );

            result = mergeEntities(result, affectedEntities);
          }
        }

        if (mnEntry.operatorPayoutAddress) {
          const mnEntryWithChangedOperatorPayoutAddress = previousMNList
            .find((previousMnListEntry) => (
              previousMnListEntry.proRegTxHash === mnEntry.proRegTxHash
              && previousMnListEntry.operatorPayoutAddress !== mnEntry.operatorPayoutAddress
            ));

          if (mnEntryWithChangedOperatorPayoutAddress) {
            const newOperatorPayoutAddress = Address.fromString(mnEntry.operatorPayoutAddress);

            const { operatorPayoutAddress } = mnEntryWithChangedOperatorPayoutAddress;

            const previousOperatorPayoutScript = operatorPayoutAddress
              ? new Script(Address.fromString(operatorPayoutAddress))
              : undefined;

            const affectedEntities = await handleUpdatedScriptPayout(
              createOperatorIdentifier(mnEntry),
              new Script(newOperatorPayoutAddress),
              blockInfo,
              previousOperatorPayoutScript,
            );

            result = mergeEntities(result, affectedEntities);
          }
        }
      }
    }

    // Create identities and shares for new masternodes

    for (const newMasternodeEntry of newMasternodes) {
      const affectedEntities = await handleNewMasternode(
        newMasternodeEntry,
        dataContract,
        blockInfo,
      );

      result = mergeEntities(result, affectedEntities);
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

      const affectedEntities = await handleRemovedMasternode(
        masternodeIdentifier,
        dataContract,
        blockInfo,
      );

      result = mergeEntities(result, affectedEntities);
    }

    result.fromHeight = lastSyncedCoreHeight;
    result.toHeight = coreHeight;

    await lastSyncedCoreHeightRepository.store(coreHeight, {
      useTransaction: true,
    });

    return result;
  }

  return synchronizeMasternodeIdentities;
}

module.exports = synchronizeMasternodeIdentitiesFactory;
