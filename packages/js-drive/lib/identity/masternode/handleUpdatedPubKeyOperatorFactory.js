const { hash } = require('@dashevo/dpp/lib/util/hash');
const Identifier = require('@dashevo/dpp/lib/identifier/Identifier');
const IdentityPublicKey = require('@dashevo/dpp/lib/identity/IdentityPublicKey');
const Transaction = require('@dashevo/dashcore-lib/lib/transaction');

/**
 *
 * @param {DashPlatformProtocol} transactionalDpp
 * @param {DriveStateRepository|CachedStateRepositoryDecorator} transactionalStateRepository
 * @param {createMasternodeIdentity} createMasternodeIdentity
 * @param {Identifier} masternodeRewardSharesContractId
 * @return {handleUpdatedPubKeyOperator}
 */
function handleUpdatedPubKeyOperatorFactory(
  transactionalDpp,
  transactionalStateRepository,
  createMasternodeIdentity,
  masternodeRewardSharesContractId,
) {
  /**
   * @typedef handleUpdatedPubKeyOperator
   * @param {SimplifiedMNListEntry} masternodeEntry
   * @param {SimplifiedMNListEntry} previousMasternodeEntry
   * @param {DataContract} dataContract
   * @return {Promise<{
   *            create: Document[],
   *            delete: Document[],
   * }>}
   */
  async function handleUpdatedPubKeyOperator(
    masternodeEntry,
    previousMasternodeEntry,
    dataContract,
  ) {
    const rawTransaction = await transactionalStateRepository
      .fetchTransaction(masternodeEntry.proRegTxHash);

    const { extraPayload: proRegTxPayload } = new Transaction(rawTransaction.data);

    // we need to crate reward shares only if it's enabled in proRegTx
    const documentsToCreate = [];
    let documentsToDelete = [];

    if (proRegTxPayload.operatorReward > 0) {
      const proRegTxHash = Buffer.from(masternodeEntry.proRegTxHash, 'hex');
      const pubKeyOperator = Buffer.from(proRegTxPayload.pubKeyOperator, 'hex');

      const operatorIdentityHash = hash(
        Buffer.concat([
          proRegTxHash,
          pubKeyOperator,
        ]),
      );

      const operatorIdentityId = Identifier.from(operatorIdentityHash);

      const operatorIdentity = await transactionalStateRepository.fetchIdentity(operatorIdentityId);

      //  Create an identity for operator if there is no identity exist with the same ID
      if (operatorIdentity === null) {
        await createMasternodeIdentity(
          operatorIdentityId,
          pubKeyOperator,
          IdentityPublicKey.TYPES.BLS12_381,
        );
      }

      // Create a document in rewards data contract with percentage defined
      // in corresponding ProRegTx
      documentsToCreate.push(transactionalDpp.document.create(
        dataContract,
        Identifier.from(
          hash(proRegTxHash),
        ),
        'rewardShares',
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

      let startAfter;
      let fetchedDocuments;
      const limit = 100;

      do {
        fetchedDocuments = await transactionalStateRepository.fetchDocuments(
          masternodeRewardSharesContractId,
          'rewardShare',
          {
            limit,
            startAfter,
            where: [
              ['$ownerId', '==', Identifier.from(proRegTxHash)],
              ['payToId', '==', previousOperatorIdentityId],
            ],
          },
        );
        documentsToDelete = documentsToDelete.concat(fetchedDocuments);
        startAfter = fetchedDocuments.length > 0
          ? fetchedDocuments[fetchedDocuments.length - 1].id : undefined;
      } while (fetchedDocuments.length === limit);
    }

    return {
      create: documentsToCreate,
      delete: documentsToDelete,
    };
  }

  return handleUpdatedPubKeyOperator;
}

module.exports = handleUpdatedPubKeyOperatorFactory;
