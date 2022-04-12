const IdentityPublicKey = require('@dashevo/dpp/lib/identity/IdentityPublicKey');
const identitySchema = require('@dashevo/dpp/schema/identity/identity.json');

/**
 *
 * @param {DriveStateRepository|CachedStateRepositoryDecorator} transactionalStateRepository
 * @param {getWithdrawPubKeyTypeFromPayoutScript} getWithdrawPubKeyTypeFromPayoutScript
 * @returns {handleUpdatedScriptPayout}
 */
function handleUpdatedScriptPayoutFactory(
  transactionalStateRepository,
  getWithdrawPubKeyTypeFromPayoutScript,
) {
  /**
   * @typedef handleUpdatedScriptPayout
   * @param {Identifier} identityId
   * @param {Buffer} newPubKeyData
   * @param {Buffer} previousPubKeyData
   * @returns {Promise<void>}
   */
  async function handleUpdatedScriptPayout(
    identityId,
    newPubKeyData,
    previousPubKeyData,
  ) {
    const identity = await transactionalStateRepository.fetchIdentity(identityId);
    identity.setRevision(identity.getRevision() + 1);
    let identityPublicKeys = identity
      .getPublicKeys();

    if (identityPublicKeys.length === identitySchema.properties.publicKeys.maxItems) {
      // do not add new public key
      return;
    }

    const maxId = identity.getPublicKeyById();

    // disable previous
    identityPublicKeys = identityPublicKeys.map((pk) => {
      if (Buffer.compare(pk.getData(), previousPubKeyData)) {
        pk.setDisabledAt(new Date().getTime());
      }

      return pk;
    });

    // add new
    const withdrawPubKeyType = getWithdrawPubKeyTypeFromPayoutScript(newPubKeyData);

    const newWithdrawalIdentityPublicKey = new IdentityPublicKey()
      .setId(maxId + 1)
      .setType(withdrawPubKeyType)
      .setData(Buffer.from(newPubKeyData))
      .setPurpose(IdentityPublicKey.PURPOSES.WITHDRAW)
      .setSecurityLevel(IdentityPublicKey.PURPOSES.MASTER);

    identityPublicKeys.push(
      newWithdrawalIdentityPublicKey,
    );

    identity.setPublicKeys(identityPublicKeys);

    await transactionalStateRepository.storeIdentity(identity);

    const publicKeyHash = newWithdrawalIdentityPublicKey.hash();

    await transactionalStateRepository.storeIdentityPublicKeyHashes(
      identity.getId(),
      [publicKeyHash],
    );
  }

  return handleUpdatedScriptPayout;
}

module.exports = handleUpdatedScriptPayoutFactory;
