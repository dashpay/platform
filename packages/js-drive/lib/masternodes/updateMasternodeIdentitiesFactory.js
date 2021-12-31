const { hash } = require('@dashevo/dpp/lib/util/hash');
const IdentityPublicKey = require('@dashevo/dpp/lib/identity/IdentityPublicKey');
const Transaction = require('@dashevo/dashcore-lib/lib/transaction');
const Identifier = require('@dashevo/dpp/lib/identifier/Identifier');

/**
 *
 * @param {DashPlatformProtocol} transactionalDpp
 * @param {DriveStateRepository|CachedStateRepositoryDecorator} stateRepository
 * @param {createIdentity} createIdentity
 * @return {updateMasternodeIdentities}
 */
function updateMasternodeIdentitiesFactory(
  transactionalDpp,
  stateRepository,
  createIdentity,
) {
  /**
   * @typedef updateMasternodeIdentities
   * @param {SimplifiedMasternodeList} simplifiedMasternodeList
   * @param {number} latestRequestedHeight
   * @return Promise<void>
   */
  async function updateMasternodeIdentities(
    simplifiedMasternodeList,
    latestRequestedHeight,
  ) {
    const documentsToCreate = [];
    let documentsToDelete = [];

    const previousMNList = simplifiedMasternodeList.getStore()
      .getSMLbyHeight(latestRequestedHeight)
      .mnList;
    const currentMNList = simplifiedMasternodeList.getStore()
      .getCurrentSML()
      .mnList;

    // new masternode is registered
    const newMasternodes = currentMNList
      .filter(
        // eslint-disable-next-line max-len
        (currentMnListEntry) => (!previousMNList.find((previousMnListEntry) => previousMnListEntry.proRegTxHash === currentMnListEntry.proRegTxHash))
      );

    await Promise.all(
      newMasternodes.map(async (masternodeEntry) => {
        const rawTransaction = await stateRepository
          .fetchTransaction(masternodeEntry.proRegTxHash);

        const { extraPayload: proRegTxInfo } = new Transaction(rawTransaction.data);

        // Create a masternode identity
        const masternodeIdentityId = Buffer.from(masternodeEntry.proRegTxHash, 'hex');

        await createIdentity(
          masternodeIdentityId,
          Buffer.from(proRegTxInfo.keyIdOwner, 'hex'),
          IdentityPublicKey.TYPES.ECDSA_HASH160,
        );

        if (proRegTxInfo.operatorReward > 0) {
          // Create an identity for operator
          const operatorIdentityId = hash(Buffer.concat(
            masternodeIdentityId,
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
            Identifier.from(masternodeEntry.proRegTxHash, 'hex'),
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

          const documents = await stateRepository.fetchDocuments(
            contractId,
            'masternodeRewardShares',
            {
              where: [
                ['$ownerId', '===', Identifier.from(masternodeEntry.proRegTxHash, 'hex')],
                ['payToId', '===', previousOperatorIdentityId],
              ],
            },
          );
          documentsToDelete = documentsToDelete.concat(documents);

          // Create a document in rewards data contract with percentage defined
          // in corresponding ProRegTx
          documentsToCreate.push(transactionalDpp.document.create(
            contract,
            Identifier.from(masternodeEntry.proRegTxHash, 'hex'),
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
      .filter((previousMnListEntry) =>
        // eslint-disable-next-line max-len,implicit-arrow-linebreak
        (!currentMNList.find((currentMnListEntry) => currentMnListEntry.proRegTxHash === previousMnListEntry.proRegTxHash)))
      .concat(currentMNList.filter((currentMnListEntry) => !currentMnListEntry.isValid));

    await Promise.all(
      disappearedOrInvalidMasterNodes.map(async (masternodeEntry) => {
        //  Delete documents belongs to masternode identity (ownerId) from rewards contract
        const documents = await stateRepository.fetchDocuments(
          contractId,
          'masternodeRewardShares',
          {
            where: [
              ['$ownerId', '===', Identifier.from(masternodeEntry.proRegTxHash, 'hex')],
            ],
          },
        );
        documentsToDelete = documentsToDelete.concat(documents);
      }),
    );

    const chunkedDocuments = [];

    const maxLength = Math.max(documentsToCreate.length, documentsToDelete.length);
    const chunk = MAX_BATCH_LENGTH;

    for (let i = 0; i < maxLength; i += chunk) {
      const documents = {};

      if (documentsToCreate.length > i) {
        documents.create = documentsToCreate.slice(i, i + chunk);
      }

      if (documentsToDelete.length > i) {
        documents.delete = documentsToDelete.slice(i, i + chunk);
      }

      chunkedDocuments.push(documents);
    }

    await Promise.all(
      chunkedDocuments.map(async (documentsChunk) => {
        const documentsBatchTransition = transactionalDpp.document.createStateTransition(
          documentsChunk,
        );

        await transactionalDpp.stateTransition.apply(documentsBatchTransition);
      }),
    );
  }

  return updateMasternodeIdentities;
}

module.exports = updateMasternodeIdentitiesFactory;
