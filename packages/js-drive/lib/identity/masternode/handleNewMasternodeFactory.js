const Identifier = require('@dashevo/dpp/lib/identifier/Identifier');
const IdentityPublicKey = require('@dashevo/dpp/lib/identity/IdentityPublicKey');
const Address = require('@dashevo/dashcore-lib/lib/address');
const Script = require('@dashevo/dashcore-lib/lib/script');
const createOperatorIdentifier = require('./createOperatorIdentifier');
const createVotingIdentifier = require('./createVotingIdentifier');

/**
 *
 * @param {DashPlatformProtocol} transactionalDpp
 * @param {createMasternodeIdentity} createMasternodeIdentity
 * @param {createRewardShareDocument} createRewardShareDocument
 * @param {fetchTransaction} fetchTransaction
 * @return {handleNewMasternode}
 */
function handleNewMasternodeFactory(
  transactionalDpp,
  createMasternodeIdentity,
  createRewardShareDocument,
  fetchTransaction,
) {
  /**
   * @typedef handleNewMasternode
   * @param {SimplifiedMNListEntry} masternodeEntry
   * @param {DataContract} dataContract
   * @param {BlockInfo} blockInfo
   * @return {Promise<{
   *  createdEntities: Array<Identity|Document>,
   *  updatedEntities: Array<Identity>,
   *  removedEntities: Array<Document>,
   * }>}
   */
  async function handleNewMasternode(masternodeEntry, dataContract, blockInfo) {
    const createdEntities = [];

    const { extraPayload: proRegTxPayload } = await fetchTransaction(masternodeEntry.proRegTxHash);

    const proRegTxHash = Buffer.from(masternodeEntry.proRegTxHash, 'hex');

    let payoutScript;

    if (masternodeEntry.payoutAddress) {
      const payoutAddress = Address.fromString(masternodeEntry.payoutAddress);
      payoutScript = new Script(payoutAddress);
    }

    // Create a masternode identity
    const masternodeIdentifier = Identifier.from(
      proRegTxHash,
    );

    const ownerPublicKeyHash = Buffer.from(proRegTxPayload.keyIDOwner, 'hex').reverse();

    createdEntities.push(
      await createMasternodeIdentity(
        blockInfo,
        masternodeIdentifier,
        ownerPublicKeyHash,
        IdentityPublicKey.TYPES.ECDSA_HASH160,
        payoutScript,
      ),
    );

    // we need to crate reward shares only if it's enabled in proRegTx

    if (proRegTxPayload.operatorReward > 0) {
      const operatorPubKey = Buffer.from(masternodeEntry.pubKeyOperator, 'hex');

      let operatorPayoutScript;
      if (masternodeEntry.operatorPayoutAddress) {
        const operatorPayoutAddress = Address.fromString(masternodeEntry.operatorPayoutAddress);
        operatorPayoutScript = new Script(operatorPayoutAddress);
      }

      const operatorIdentifier = createOperatorIdentifier(masternodeEntry);

      createdEntities.push(
        await createMasternodeIdentity(
          blockInfo,
          operatorIdentifier,
          operatorPubKey,
          IdentityPublicKey.TYPES.BLS12_381,
          operatorPayoutScript,
        ),
      );

      // Create a document in rewards data contract with percentage
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
    }

    const votingPubKeyHash = Buffer.from(proRegTxPayload.keyIDVoting, 'hex').reverse();

    // don't need to create a separate Identity in case we don't have
    // voting public key (keyIDVoting === keyIDOwner)
    if (!votingPubKeyHash.equals(ownerPublicKeyHash)) {
      const votingIdentifier = createVotingIdentifier(masternodeEntry);

      createdEntities.push(
        await createMasternodeIdentity(
          blockInfo,
          votingIdentifier,
          votingPubKeyHash,
          IdentityPublicKey.TYPES.ECDSA_HASH160,
        ),
      );
    }

    return {
      createdEntities,
      updatedEntities: [],
      removedEntities: [],
    };
  }

  return handleNewMasternode;
}

module.exports = handleNewMasternodeFactory;
