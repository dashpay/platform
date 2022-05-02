const Identifier = require('@dashevo/dpp/lib/identifier/Identifier');
const IdentityPublicKey = require('@dashevo/dpp/lib/identity/IdentityPublicKey');
const createOperatorIdentifier = require('./createOperatorIdentifier');

/**
 *
 * @param {DashPlatformProtocol} transactionalDpp
 * @param {DriveStateRepository|CachedStateRepositoryDecorator} transactionalStateRepository
 * @param {createMasternodeIdentity} createMasternodeIdentity
 * @param {Identifier} masternodeRewardSharesContractId
 * @param {createRewardShareDocument} createRewardShareDocument
 * @param {DocumentRepository} documentRepository
 * @param {fetchTransaction} fetchTransaction
 * @return {handleUpdatedPubKeyOperator}
 */
function handleUpdatedPubKeyOperatorFactory(
  transactionalDpp,
  transactionalStateRepository,
  createMasternodeIdentity,
  masternodeRewardSharesContractId,
  createRewardShareDocument,
  documentRepository,
  fetchTransaction,
) {
  /**
   * @typedef handleUpdatedPubKeyOperator
   * @param {SimplifiedMNListEntry} masternodeEntry
   * @param {SimplifiedMNListEntry} previousMasternodeEntry
   * @param {DataContract} dataContract
   */
  async function handleUpdatedPubKeyOperator(
    masternodeEntry,
    previousMasternodeEntry,
    dataContract,
  ) {
    const { extraPayload: proRegTxPayload } = await fetchTransaction(masternodeEntry.proRegTxHash);

    // we need to crate reward shares only if it's enabled in proRegTx
    if (proRegTxPayload.operatorReward === 0) {
      return;
    }

    const proRegTxHash = Buffer.from(masternodeEntry.proRegTxHash, 'hex');
    const operatorPublicKey = Buffer.from(masternodeEntry.pubKeyOperator, 'hex');

    const operatorIdentifier = createOperatorIdentifier(masternodeEntry);

    const operatorIdentity = await transactionalStateRepository.fetchIdentity(operatorIdentifier);

    //  Create an identity for operator if there is no identity exist with the same ID
    if (operatorIdentity === null) {
      await createMasternodeIdentity(
        operatorIdentifier,
        operatorPublicKey,
        IdentityPublicKey.TYPES.BLS12_381,
      );
    }

    // Create a document in rewards data contract with percentage defined
    // in corresponding ProRegTx

    const masternodeIdentifier = Identifier.from(
      proRegTxHash,
    );

    await createRewardShareDocument(
      dataContract,
      masternodeIdentifier,
      operatorIdentifier,
      proRegTxPayload.operatorReward,
    );

    // Delete document from reward shares data contract with ID corresponding to the
    // masternode identity (ownerId) and previous operator identity (payToId)

    const previousOperatorIdentifier = createOperatorIdentifier(previousMasternodeEntry);

    const previousDocumentsResult = await documentRepository.find(
      dataContract,
      'rewardShare',
      {
        where: [
          ['$ownerId', '==', masternodeIdentifier],
          ['payToId', '==', previousOperatorIdentifier],
        ],
      },
      true,
    );

    if (!previousDocumentsResult.isEmpty()) {
      const [previousDocument] = previousDocumentsResult.getValue();

      await documentRepository.delete(
        dataContract,
        'rewardShare',
        previousDocument.getId(),
        true,
      );
    }
  }

  return handleUpdatedPubKeyOperator;
}

module.exports = handleUpdatedPubKeyOperatorFactory;
