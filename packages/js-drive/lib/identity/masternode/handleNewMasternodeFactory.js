const Identifier = require('@dashevo/dpp/lib/identifier/Identifier');
const IdentityPublicKey = require('@dashevo/dpp/lib/identity/IdentityPublicKey');
const Address = require('@dashevo/dashcore-lib/lib/address');
const Script = require('@dashevo/dashcore-lib/lib/script');
const createOperatorIdentifier = require('./createOperatorIdentifier');

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
   */
  async function handleNewMasternode(masternodeEntry, dataContract) {
    const { extraPayload: proRegTxPayload } = await fetchTransaction(masternodeEntry.proRegTxHash);

    const proRegTxHash = Buffer.from(masternodeEntry.proRegTxHash, 'hex');
    const payoutAddress = Address.fromString(masternodeEntry.operatorPayoutAddress);
    const payoutPubKey = new Script(payoutAddress).toBuffer();

    // Create a masternode identity
    const masternodeIdentifier = Identifier.from(
      proRegTxHash,
    );

    const publicKey = Buffer.from(proRegTxPayload.keyIDOwner, 'hex').reverse();

    await createMasternodeIdentity(
      masternodeIdentifier,
      publicKey,
      IdentityPublicKey.TYPES.ECDSA_HASH160,
      payoutPubKey,
    );

    // we need to crate reward shares only if it's enabled in proRegTx

    if (proRegTxPayload.operatorReward > 0) {
      const operatorPubKey = Buffer.from(masternodeEntry.pubKeyOperator, 'hex');
      const operatorPayoutAddress = Address.fromString(masternodeEntry.operatorPayoutAddress);
      const operatorPayoutPubKey = new Script(operatorPayoutAddress).toBuffer();

      const operatorIdentifier = createOperatorIdentifier(masternodeEntry);

      await createMasternodeIdentity(
        operatorIdentifier,
        operatorPubKey,
        IdentityPublicKey.TYPES.BLS12_381,
        operatorPayoutPubKey,
      );

      // Create a document in rewards data contract with percentage
      await createRewardShareDocument(
        dataContract,
        masternodeIdentifier,
        operatorIdentifier,
        proRegTxPayload.operatorReward,
      );
    }
  }

  return handleNewMasternode;
}

module.exports = handleNewMasternodeFactory;
