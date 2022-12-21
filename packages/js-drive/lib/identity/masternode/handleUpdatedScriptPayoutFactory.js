const IdentityPublicKey = require('@dashevo/dpp/lib/identity/IdentityPublicKey');
const identitySchema = require('@dashevo/dpp/schema/identity/identity.json');

/**
 *
 * @param {IdentityStoreRepository} identityRepository
 * @param {PublicKeyToIdentitiesStoreRepository} publicKeyToIdentitiesRepository
 * @param {getWithdrawPubKeyTypeFromPayoutScript} getWithdrawPubKeyTypeFromPayoutScript
 * @param {getPublicKeyFromPayoutScript} getPublicKeyFromPayoutScript
 * @returns {handleUpdatedScriptPayout}
 */
function handleUpdatedScriptPayoutFactory(
  identityRepository,
  publicKeyToIdentitiesRepository,
  getWithdrawPubKeyTypeFromPayoutScript,
  getPublicKeyFromPayoutScript,
) {
  /**
   * @typedef handleUpdatedScriptPayout
   * @param {Identifier} identityId
   * @param {Script} newPayoutScript
   * @param {BlockInfo} blockInfo
   * @param {Script} [previousPayoutScript]
   * @return {Promise<{
   *  createdEntities: Array<Identity|Document>,
   *  updatedEntities: Array<Identity>,
   *  removedEntities: Array<Document>,
   * }>}
   */
  async function handleUpdatedScriptPayout(
    identityId,
    newPayoutScript,
    blockInfo,
    previousPayoutScript,
  ) {
    const result = {
      createdEntities: [],
      updatedEntities: [],
      removedEntities: [],
    };

    const identityResult = await identityRepository.fetch(identityId, { useTransaction: true });

    const identity = identityResult.getValue();

    identity.setRevision(identity.getRevision() + 1);

    let identityPublicKeys = identity
      .getPublicKeys();

    if (identityPublicKeys.length === identitySchema.properties.publicKeys.maxItems) {
      // do not add new public key
      return result;
    }

    // disable previous
    if (previousPayoutScript) {
      const previousPubKeyType = getWithdrawPubKeyTypeFromPayoutScript(previousPayoutScript);
      const previousPubKeyData = getPublicKeyFromPayoutScript(
        previousPayoutScript,
        previousPubKeyType,
      );

      identityPublicKeys = identityPublicKeys.map((pk) => {
        if (pk.getData().equals(previousPubKeyData)) {
          pk.setDisabledAt(
            blockInfo.timeMs,
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

    await identityRepository.update(identity, { useTransaction: true });

    const publicKeyHash = newWithdrawalIdentityPublicKey.hash();

    await publicKeyToIdentitiesRepository.store(
      publicKeyHash,
      identity.getId(),
      { useTransaction: true },
    );

    result.updatedEntities.push(identity);

    return result;
  }

  return handleUpdatedScriptPayout;
}

module.exports = handleUpdatedScriptPayoutFactory;
