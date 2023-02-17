const Identifier = require('@dashevo/dpp/lib/identifier/Identifier');
const IdentityPublicKey = require('@dashevo/dpp/lib/identity/IdentityPublicKey');
const Address = require('@dashevo/dashcore-lib/lib/address');
const Script = require('@dashevo/dashcore-lib/lib/script');
const createOperatorIdentifier = require('./createOperatorIdentifier');

/**
 *
 * @param {DashPlatformProtocol} transactionalDpp
 * @param {createMasternodeIdentity} createMasternodeIdentity
 * @param {Identifier} masternodeRewardSharesContractId
 * @param {createRewardShareDocument} createRewardShareDocument
 * @param {DocumentRepository} documentRepository
 * @param {IdentityStoreRepository} identityRepository
 * @param {fetchTransaction} fetchTransaction
 * @return {handleUpdatedPubKeyOperator}
 */
function handleUpdatedPubKeyOperatorFactory(
  transactionalDpp,
  createMasternodeIdentity,
  masternodeRewardSharesContractId,
  createRewardShareDocument,
  documentRepository,
  identityRepository,
  fetchTransaction,
) {
  /**
   * @typedef handleUpdatedPubKeyOperator
   * @param {SimplifiedMNListEntry} masternodeEntry
   * @param {SimplifiedMNListEntry} previousMasternodeEntry
   * @param {DataContract} dataContract
   * @param {BlockInfo} blockInfo
   * @return Promise<{
   *  createdEntities: Array<Identity|Document>,
   *  removedEntities: Array<Document>,
   * }>
   */
  async function handleUpdatedPubKeyOperator(
    masternodeEntry,
    previousMasternodeEntry,
    dataContract,
    blockInfo,
  ) {
    const createdEntities = [];
    const removedEntities = [];

    const { extraPayload: proRegTxPayload } = await fetchTransaction(masternodeEntry.proRegTxHash);

    // we need to crate reward shares only if it's enabled in proRegTx
    if (proRegTxPayload.operatorReward === 0) {
      return {
        createdEntities,
        removedEntities,
      };
    }

    const proRegTxHash = Buffer.from(masternodeEntry.proRegTxHash, 'hex');
    const operatorPublicKey = Buffer.from(masternodeEntry.pubKeyOperator, 'hex');

    const operatorIdentifier = createOperatorIdentifier(masternodeEntry);

    const operatorIdentityResult = await identityRepository.fetch(
      operatorIdentifier,
      { useTransaction: true },
    );

    let operatorPayoutPubKey;
    if (masternodeEntry.operatorPayoutAddress) {
      const operatorPayoutAddress = Address.fromString(masternodeEntry.operatorPayoutAddress);
      operatorPayoutPubKey = new Script(operatorPayoutAddress);
    }

    //  Create an identity for operator if there is no identity exist with the same ID
    if (operatorIdentityResult.isNull()) {
      createdEntities.push(
        await createMasternodeIdentity(
          blockInfo,
          operatorIdentifier,
          operatorPublicKey,
          IdentityPublicKey.TYPES.BLS12_381,
          operatorPayoutPubKey,
        ),
      );
    }

    // Create a document in rewards data contract with percentage defined
    // in corresponding ProRegTx

    const masternodeIdentifier = Identifier.from(
      proRegTxHash,
    );

    const rewardShareDocument = await createRewardShareDocument(
      dataContract,
      masternodeIdentifier,
      operatorIdentifier,
      proRegTxPayload.operatorReward,
      blockInfo,
    );

    if (rewardShareDocument) {
      createdEntities.push(rewardShareDocument);
    }

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
        useTransaction: true,
      },
    );

    if (!previousDocumentsResult.isEmpty()) {
      const [previousDocument] = previousDocumentsResult.getValue();

      await documentRepository.delete(
        dataContract,
        'rewardShare',
        previousDocument.getId(),
        blockInfo,
        { useTransaction: true },
      );

      removedEntities.push(previousDocument);
    }

    return {
      createdEntities,
      removedEntities,
    };
  }

  return handleUpdatedPubKeyOperator;
}

module.exports = handleUpdatedPubKeyOperatorFactory;
