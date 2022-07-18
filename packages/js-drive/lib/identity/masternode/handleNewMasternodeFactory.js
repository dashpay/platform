const Identifier = require('@dashevo/dpp/lib/identifier/Identifier');
const IdentityPublicKey = require('@dashevo/dpp/lib/identity/IdentityPublicKey');
const Address = require('@dashevo/dashcore-lib/lib/address');
const Script = require('@dashevo/dashcore-lib/lib/script');
const createOperatorIdentifier = require('./createOperatorIdentifier');
const createVotingIdentifier = require('./createVotingIdentifier');

/**
 *
 * @param {DashPlatformProtocol} transactionalDpp
 * @param {DriveStateRepository|CachedStateRepositoryDecorator} transactionalStateRepository
 * @param {createMasternodeIdentity} createMasternodeIdentity
 * @param {createRewardShareDocument} createRewardShareDocument
 * @param {fetchTransaction} fetchTransaction
 * @return {handleNewMasternode}
 */
function handleNewMasternodeFactory(
  transactionalDpp,
  transactionalStateRepository,
  createMasternodeIdentity,
  createRewardShareDocument,
  fetchTransaction,
) {
  /**
   * @typedef handleNewMasternode
   * @param {SimplifiedMNListEntry} masternodeEntry
   * @param {DataContract} dataContract
   * @return Promise<Array<Identity|Document>>
   */
  async function handleNewMasternode(masternodeEntry, dataContract) {
    const result = [];

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

    const publicKeyOwner = Buffer.from(proRegTxPayload.keyIDOwner, 'hex').reverse();

    result.push(
      await createMasternodeIdentity(
        masternodeIdentifier,
        publicKeyOwner,
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

      result.push(
        await createMasternodeIdentity(
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
      );

      if (rewardShareDocument) {
        result.push(rewardShareDocument);
      }
    }

    const votingPubKeyHash = Buffer.from(proRegTxPayload.keyIDVoting, 'hex').reverse();

    if (!votingPubKeyHash.equals(publicKeyOwner)) {
      const votingIdentifier = createVotingIdentifier(masternodeEntry);

      result.push(
        await createMasternodeIdentity(
          votingIdentifier,
          votingPubKeyHash,
          IdentityPublicKey.TYPES.ECDSA_HASH160,
        ),
      );
    }

    return result;
  }

  return handleNewMasternode;
}

module.exports = handleNewMasternodeFactory;
