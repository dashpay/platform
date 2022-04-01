const IdentityPublicKey = require('@dashevo/dpp/lib/identity/IdentityPublicKey');
const Script = require('@dashevo/dashcore-lib/lib/script');

/**
 *
 * @param {DriveStateRepository|CachedStateRepositoryDecorator} transactionalStateRepository
 * @param {string} network
 * @returns {handleUpdatedScriptPayout}
 */
function handleUpdatedScriptPayoutFactory(
  transactionalStateRepository,
  network,
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
    identity.setRevision(identity.getRevision() + 1); // ????
    let identityPublicKeys = identity
      .getPublicKeys();

    const maxId = identityPublicKeys.reduce(
      (result, pk) => (result > pk.getId() ? result : pk.getId()), 0,
    );

    // disable previous
    identityPublicKeys = identityPublicKeys.map((pk) => {
      if (Buffer.compare(pk.getData(), previousPubKeyData)) {
        pk.setDisabledAt(new Date().getTime());
      }

      return pk;
    });

    // add new
    const script = new Script(newPubKeyData);

    const withdrawPubKeyType = script.toAddress(network).isPayToScriptHash()
      ? IdentityPublicKey.TYPES.BIP13_SCRIPT_HASH
      : IdentityPublicKey.TYPES.ECDSA_HASH160;

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
