const IdentityPublicKey = require('@dashevo/dpp/lib/identity/IdentityPublicKey');
const identitySchema = require('@dashevo/dpp/schema/identity/identity.json');

/**
 *
 * @param {DriveStateRepository|CachedStateRepositoryDecorator} transactionalStateRepository
 * @param {BlockExecutionContext} latestBlockExecutionContext
 * @param {getWithdrawPubKeyTypeFromPayoutScript} getWithdrawPubKeyTypeFromPayoutScript
 * @param {getPublicKeyFromPayoutScript} getPublicKeyFromPayoutScript
 * @returns {handleUpdatedScriptPayout}
 */
function handleUpdatedScriptPayoutFactory(
  transactionalStateRepository,
  latestBlockExecutionContext,
  getWithdrawPubKeyTypeFromPayoutScript,
  getPublicKeyFromPayoutScript,
) {
  /**
   * @typedef handleUpdatedScriptPayout
   * @param {Identifier} identityId
   * @param {Script} newPayoutScript
   * @param {Script} [previousPayoutScript]
   * @returns {Promise<void>}
   */
  async function handleUpdatedScriptPayout(
    identityId,
    newPayoutScript,
    previousPayoutScript,
  ) {
    const identity = await transactionalStateRepository.fetchIdentity(identityId);
    identity.setRevision(identity.getRevision() + 1);
    let identityPublicKeys = identity
      .getPublicKeys();

    if (identityPublicKeys.length === identitySchema.properties.publicKeys.maxItems) {
      // do not add new public key
      return;
    }

    // disable previous
    if (previousPayoutScript) {
      const previousPubKeyType = getWithdrawPubKeyTypeFromPayoutScript(previousPayoutScript);
      const previousPubKeyData = getPublicKeyFromPayoutScript(
        previousPayoutScript,
        previousPubKeyType,
      );
      const time = latestBlockExecutionContext.getTime();

      identityPublicKeys = identityPublicKeys.map((pk) => {
        if (pk.getData().equals(previousPubKeyData)) {
          pk.setDisabledAt(
            time.seconds * 1000,
          );
        }

        return pk;
      });
    }

    // add new
    const withdrawPubKeyType = getWithdrawPubKeyTypeFromPayoutScript(newPayoutScript);
    const pubKeyData = getPublicKeyFromPayoutScript(newPayoutScript, withdrawPubKeyType);

    const newWithdrawalIdentityPublicKey = new IdentityPublicKey()
      .setId(identity.getPublicKeyMaxId() + 1)
      .setType(withdrawPubKeyType)
      .setData(pubKeyData)
      .setPurpose(IdentityPublicKey.PURPOSES.WITHDRAW)
      .setSecurityLevel(IdentityPublicKey.SECURITY_LEVELS.MASTER);

    identityPublicKeys.push(
      newWithdrawalIdentityPublicKey,
    );

    identity.setPublicKeys(identityPublicKeys);

    await transactionalStateRepository.updateIdentity(identity);

    const publicKeyHash = newWithdrawalIdentityPublicKey.hash();

    await transactionalStateRepository.storeIdentityPublicKeyHashes(
      identity.getId(),
      [publicKeyHash],
    );
  }

  return handleUpdatedScriptPayout;
}

module.exports = handleUpdatedScriptPayoutFactory;
