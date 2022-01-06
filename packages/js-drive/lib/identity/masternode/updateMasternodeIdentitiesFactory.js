const { hash } = require('@dashevo/dpp/lib/util/hash');
const IdentityPublicKey = require('@dashevo/dpp/lib/identity/IdentityPublicKey');
const Transaction = require('@dashevo/dashcore-lib/lib/transaction');
const Identifier = require('@dashevo/dpp/lib/identifier/Identifier');
const masternodeRewardSharesContractDocuments = require('@dashevo/masternode-reward-shares-contract/schema/masternode-reward-shares-documents.json');

/**
 *
 * @param {DashPlatformProtocol} transactionalDpp
 * @param {DriveStateRepository|CachedStateRepositoryDecorator} stateRepository
 * @param {createMasternodeIdentity} createMasternodeIdentity
 * @param {SimplifiedMasternodeList} simplifiedMasternodeList
 * @return {updateMasternodeIdentities}
 */
function updateMasternodeIdentitiesFactory(
  transactionalDpp,
  stateRepository,
  createMasternodeIdentity,
  simplifiedMasternodeList,
) {
  let lastSyncedCoreHeight = 0;

  /**
   * @param {SimplifiedMNListEntry} masternodeEntry
   * @return {Promise<Document>}
   */
  async function handleNewMasternode(masternodeEntry) {
    const rawTransaction = await stateRepository
      .fetchTransaction(masternodeEntry.proRegTxHash);

    const { extraPayload: proRegTxPayload } = new Transaction(rawTransaction.data);

    // Create a masternode identity
    const masternodeIdentityId = Identifier.from(masternodeEntry.proRegTxHash, 'hex');

    await createMasternodeIdentity(
      masternodeIdentityId,
      Buffer.from(proRegTxPayload.keyIdOwner, 'hex'),
      IdentityPublicKey.TYPES.ECDSA_HASH160,
    );

    const documentsToCreate = [];
    const documentsToDelete = [];

    if (proRegTxPayload.operatorReward > 0) {
      const operatorPubKey = Buffer.from(proRegTxPayload.state.pubKeyOperator, 'hex');

      // Create an identity for operator
      const operatorIdentityHash = hash(
        Buffer.concat([
          masternodeIdentityId.toBuffer(),
          operatorPubKey,
        ]),
      );

      const operatorIdentityId = Identifier.from(operatorIdentityHash);

      await createMasternodeIdentity(
        operatorIdentityId,
        Buffer.from(proRegTxPayload.state.pubKeyOperator, 'hex'),
        IdentityPublicKey.TYPES.BLS12_381,
      );

      // Create a document in rewards data contract with percentage
      documentsToCreate.push(transactionalDpp.document.create(
        contract,
        Identifier.from(masternodeIdentityId),
        'masternodeRewardShares',
        {
          payToId: operatorIdentityId,
          percentage: proRegTxPayload.operatorReward,
        },
      ));
    }

    return {
      create: documentsToCreate,
      delete: documentsToDelete,
    };
  }

  /**
   * @param {SimplifiedMNListEntry} masternodeEntry
   * @param {SimplifiedMNListEntry} previousMasternodeEntry
   * @return {Promise<Document>}
   */
  async function handleUpdatedPubKeyOperator(masternodeEntry, previousMasternodeEntry) {
    const rawTransaction = await stateRepository
      .fetchTransaction(masternodeEntry.proRegTxHash);

    const { extraPayload: proRegTxPayload } = new Transaction(rawTransaction.data);

    const documentsToCreate = [];
    const documentsToDelete = [];

    if (proRegTxPayload.operatorReward > 0) {
      const proRegTxHash = Buffer.from(masternodeEntry.proRegTxHash, 'hex');

      const operatorIdentityHash = hash(
        Buffer.concat([
          proRegTxHash,
          Buffer.from(masternodeEntry.pubKeyOperator, 'hex'),
        ]),
      );

      const operatorIdentityId = Identifier.from(operatorIdentityHash);

      const operatorIdentity = await stateRepository.fetchIdentity(operatorIdentityId);

      //  Create an identity for operator if there is no identity exist with the same ID
      if (operatorIdentity === null) {
        await createMasternodeIdentity(
          operatorIdentityId,
          Buffer.from(proRegTxPayload.state.pubKeyOperator, 'hex'),
          IdentityPublicKey.TYPES.BLS12_381,
        );
      }

      // Create a document in rewards data contract with percentage defined
      // in corresponding ProRegTx
      documentsToCreate.push(transactionalDpp.document.create(
        contract,
        Identifier.from(masternodeEntry.proRegTxHash, 'hex'),
        'masternodeRewardShares',
        {
          payToId: operatorIdentityId,
          percentage: proRegTxPayload.operatorReward,
        },
      ));

      // Delete document from reward shares data contract with ID corresponding to the
      // masternode identity (ownerId) and previous operator identity (payToId)

      const previousOperatorIdentityHash = hash(
        Buffer.concat([
          proRegTxHash,
          Buffer.from(previousMasternodeEntry.pubKeyOperator, 'hex'),
        ]),
      );

      const previousOperatorIdentityId = Identifier.from(previousOperatorIdentityHash);

      const documents = await stateRepository.fetchDocuments(
        contractId,
        'rewardShare',
        {
          where: [
            ['$ownerId', '===', Identifier.from(proRegTxHash)],
            ['payToId', '===', previousOperatorIdentityId],
          ],
        },
      );

      return {
        create: documentsToCreate,
        delete: documentsToDelete,
      };
    }
  }

  /**
   * @typedef updateMasternodeIdentities
   * @param {number} coreHeight
   * @return Promise<void>
   */
  async function updateMasternodeIdentities( // TODO: Rename to synchronizeMasternodeIdentities
    coreHeight,
  ) {
    const documentsToCreate = [];
    let documentsToDelete = [];

    let newMasternodes = [];
    let masternodesWithNewPubKeyOperator = [];

    if (lastSyncedCoreHeight === 0) {
      // Create identities for all masternodes on the first sync
      newMasternodes = simplifiedMasternodeList.getStore().currentSML;
    } else {
      // Get the difference between last sync and requested core height
      const previousMNList = simplifiedMasternodeList.getStore()
        .getSMLbyHeight(lastSyncedCoreHeight)
        .mnList;

      const currentMNList = simplifiedMasternodeList.getStore()
        .getSMLbyHeight(coreHeight)
        .mnList;

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
    const ocumentsToCreate = await Promise.all(
      newMasternodes.map(handleNewMasternode)
    );

    await Promise.all(
      masternodesWithNewPubKeyOperator.map(handleUpdatedPubKeyOperator),
    );


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
