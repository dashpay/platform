const { hash } = require('@dashevo/dpp/lib/util/hash');
const IdentityPublicKey = require('@dashevo/dpp/lib/identity/IdentityPublicKey');
const Transaction = require('@dashevo/dashcore-lib/lib/transaction');

/**
 *
 * @param {DashPlatformProtocol} transactionalDpp
 * @param {DriveStateRepository|CachedStateRepositoryDecorator} stateRepository
 * @param {createIdentity} createIdentity
 * @return {manageMasternodesIdentities}
 */
function manageMasternodesIdentitiesFactory(
  transactionalDpp,
  stateRepository,
  createIdentity,
) {
  /**
   * @typedef manageMasternodesIdentities
   * @param {SimplifiedMasternodeList} simplifiedMasternodeList
   * @param {number} latestRequestedHeight
   * @return Promise<void>
   */
  async function manageMasternodesIdentities(
    simplifiedMasternodeList,
    latestRequestedHeight,
  ) {
    const documentsToCreate = [];
    const documentsToDelete = [];

    const previousMNList = simplifiedMasternodeList.getStore()
      .getSMLbyHeight(latestRequestedHeight)
      .mnList;
    const currentMNList = simplifiedMasternodeList.getStore()
      .getCurrentSML()
      .mnList;

    // new masternode is registered
    const newMasternodes = currentMNList
      .filter((currentMnListEntry) => !previousMNList
        // eslint-disable-next-line max-len
        .find((previousMnListEntry) => previousMnListEntry.proRegTxHash === currentMnListEntry.proRegTxHash));

    await Promise.all(
      newMasternodes.map(async (masternodeEntry) => {
        const rawTransaction = await stateRepository
          .fetchTransaction(masternodeEntry.proRegTxHash);

        const { extraPayload: proRegTxInfo } = new Transaction(rawTransaction.data);

        // Create a masternode identity
        await createIdentity(
          Buffer.from(masternodeEntry.proRegTxHash, 'hex'),
          Buffer.from(proRegTxInfo.keyIdOwner, 'hex'),
          IdentityPublicKey.TYPES.ECDSA_HASH160,
        );

        if (proRegTxInfo.operatorReward > 0) {
          // Create an identity for operator (charged from operator)
          const operatorIdentityId = hash(Buffer.concat(
            Buffer.from(masternodeEntry.proRegTxHash, 'hex'),
            Buffer.from(proRegTxInfo.state.pubKeyOperator, 'hex'),
          ));

          await createIdentity(
            operatorIdentityId,
            Buffer.from(proRegTxInfo.state.pubKeyOperator, 'hex'),
            IdentityPublicKey.TYPES.BLS12_381,
          );

          // Create a document in rewards data contract with percentage
          documentsToCreate.push(transactionalDpp.document.create(
            contract,
            Buffer.from(masternodeEntry.proRegTxHash, 'hex'),
            'masternodeRewardShares',
            {
              payToId: operatorIdentityId,
              percentage: proRegTxInfo.operatorReward,
            },
          ));
        }
      }),
    );

    // PubKeyOperator is changed
    const masternodesWithNewPubKeyOperator = currentMNList
      .filter((currentMnListEntry) => previousMNList
        // eslint-disable-next-line max-len
        .find((previousMnListEntry) => previousMnListEntry.proRegTxHash === currentMnListEntry.proRegTxHash && previousMnListEntry.pubKeyOperator !== currentMnListEntry.pubKeyOperator));

    await Promise.all(
      masternodesWithNewPubKeyOperator.map(async (masternodeEntry) => {
        const rawTransaction = await stateRepository
          .fetchTransaction(masternodeEntry.proRegTxHash);

        const { extraPayload: proRegTxInfo } = new Transaction(rawTransaction.data);

        if (proRegTxInfo.operatorReward > 0) {
          const operatorIdentityId = hash(Buffer.concat(
            Buffer.from(masternodeEntry.proRegTxHash, 'hex'),
            Buffer.from(proRegTxInfo.state.pubKeyOperator, 'hex'),
          ));

          const operatorIdentity = await stateRepository.fetchIdentity(operatorIdentityId);

          //  Create an identity for operator if there is no identity exist with the same ID
          if (operatorIdentity === null) {
            await createIdentity(
              operatorIdentityId,
              Buffer.from(proRegTxInfo.state.pubKeyOperator, 'hex'),
              IdentityPublicKey.TYPES.BLS12_381,
            );
          }

          // Delete document from rewards data contract with ID corresponding to the
          // masternode identity (ownerId) and previous operator identity (payToId)

          const previousOperatorData = previousMNList
            // eslint-disable-next-line max-len
            .find((previousMnListEntry) => previousMnListEntry.proRegTxHash === masternodeEntry.proRegTxHash);

          const previousOperatorIdentityId = hash(Buffer.concat(
            Buffer.from(previousOperatorData.proRegTxHash, 'hex'),
            Buffer.from(proRegTxInfo.state.pubKeyOperator, 'hex'),
          ));

          documentsToDelete.push(transactionalDpp.document.create(
            contract,
            Buffer.from(masternodeEntry.proRegTxHash, 'hex'),
            'masternodeRewardShares',
            {
              payToId: previousOperatorIdentityId,
              percentage: proRegTxInfo.operatorReward,
            },
          ));

          // Create a document in rewards data contract with percentage defined
          // in corresponding ProRegTx
          documentsToCreate.push(transactionalDpp.document.create(
            contract,
            Buffer.from(masternodeEntry.proRegTxHash, 'hex'),
            'masternodeRewardShares',
            {
              payToId: operatorIdentityId,
              percentage: proRegTxInfo.operatorReward,
            },
          ));
        }
      }),
    );

    // A masternode disappeared or is not valid
    const disappearedOrInvalidMasterNodes = previousMNList
      .filter((previousMnListEntry) => !currentMNList
        // eslint-disable-next-line max-len
        .find((currentMnListEntry) => currentMnListEntry.proRegTxHash === previousMnListEntry.proRegTxHash))
      .concat(currentMNList.filter((currentMnListEntry) => !currentMnListEntry.isValid));

    await Promise.all(
      disappearedOrInvalidMasterNodes.map(async (masternodeEntry) => {
        const rawTransaction = await stateRepository
          .fetchTransaction(masternodeEntry.proRegTxHash);

        const { extraPayload: proRegTxInfo } = new Transaction(rawTransaction.data);

        if (proRegTxInfo.operatorReward > 0) {
          const operatorIdentityId = hash(Buffer.concat(
            Buffer.from(masternodeEntry.proRegTxHash, 'hex'),
            Buffer.from(proRegTxInfo.state.pubKeyOperator, 'hex'),
          ));

          //  Delete documents belongs to masternode identity (ownerId) from rewards contract
          documentsToDelete.push(transactionalDpp.document.create(
            contract,
            Buffer.from(masternodeEntry.proRegTxHash, 'hex'),
            'masternodeRewardShares',
            {
              payToId: operatorIdentityId,
              percentage: proRegTxInfo.operatorReward,
            },
          ));
        }
      }),
    );

    const documents = {};

    if (documentsToCreate.length > 0) {
      documents.create = documentsToCreate;
    }

    if (documentsToDelete.length > 0) {
      documents.delete = documentsToDelete;
    }

    const documentsBatchTransition = transactionalDpp.document.createStateTransition(
      documents,
    );

    await transactionalDpp.stateTransition.apply(documentsBatchTransition);
  }

  return manageMasternodesIdentities;
}

module.exports = manageMasternodesIdentitiesFactory;
