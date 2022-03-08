const Identifier = require('@dashevo/dpp/lib/identifier/Identifier');
const { hash } = require('@dashevo/dpp/lib/util/hash');
const IdentityPublicKey = require('@dashevo/dpp/lib/identity/IdentityPublicKey');
const Transaction = require('@dashevo/dashcore-lib/lib/transaction');

/**
 *
 * @param {DashPlatformProtocol} transactionalDpp
 * @param {DriveStateRepository|CachedStateRepositoryDecorator} transactionalStateRepository
 * @param {createMasternodeIdentity} createMasternodeIdentity
 * @return {handleNewMasternode}
 */
function handleNewMasternodeFactory(
  transactionalDpp,
  transactionalStateRepository,
  createMasternodeIdentity,
) {
  /**
   * @typedef handleNewMasternode
   * @param {SimplifiedMNListEntry} masternodeEntry
   * @param {DataContract} dataContract
   * @return {Promise<{
   *            create: Document[],
   *            delete: Document[],
   * }>}
   */
  async function handleNewMasternode(masternodeEntry, dataContract) {
    const rawTransaction = await transactionalStateRepository
      .fetchTransaction(masternodeEntry.proRegTxHash);

    const { extraPayload: proRegTxPayload } = new Transaction(rawTransaction.data);

    // Create a masternode identity
    const masternodeIdentityId = Identifier.from(
      hash(
        Buffer.from(masternodeEntry.proRegTxHash, 'hex'),
      ),
    );

    await createMasternodeIdentity(
      masternodeIdentityId,
      Buffer.from(proRegTxPayload.keyIDOwner, 'hex').reverse(),
      IdentityPublicKey.TYPES.ECDSA_HASH160,
    );

    // we need to crate reward shares only if it's enabled in proRegTx
    const documentsToCreate = [];
    const documentsToDelete = [];

    if (proRegTxPayload.operatorReward > 0) {
      const operatorPubKey = Buffer.from(proRegTxPayload.pubKeyOperator, 'hex');

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
        operatorPubKey,
        IdentityPublicKey.TYPES.BLS12_381,
      );

      // Create a document in rewards data contract with percentage
      documentsToCreate.push(transactionalDpp.document.create(
        dataContract,
        masternodeIdentityId,
        'rewardShare',
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

  return handleNewMasternode;
}

module.exports = handleNewMasternodeFactory;
