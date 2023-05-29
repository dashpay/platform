// const { IdentityPublicKey, KeyPurpose, KeySecurityLevel } = require('@dashevo/wasm-dpp');
const identitySchema = require('@dashevo/dpp/schema/identity/identity.json');

/**
 *
 * @param {IdentityStoreRepository} identityRepository
 * @param {IdentityPublicKeyStoreRepository} identityPublicKeyRepository
 * @param {getWithdrawPubKeyTypeFromPayoutScript} getWithdrawPubKeyTypeFromPayoutScript
 * @param {getPublicKeyFromPayoutScript} getPublicKeyFromPayoutScript
 * @returns {handleUpdatedScriptPayout}
 */
function handleUpdatedScriptPayoutFactory(
  identityRepository,
  identityPublicKeyRepository,
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

    const identityPublicKeys = identity
      .getPublicKeys();

    if (identityPublicKeys.length > identitySchema.properties.publicKeys.maxItems) {
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

      const keyIds = identityPublicKeys
        .filter((pk) => pk.getData().equals(previousPubKeyData))
        .map((pk) => pk.getId());

      if (keyIds.length > 0) {
        await identityPublicKeyRepository.disable(
          identityId,
          keyIds,
          blockInfo.timeMs,
          blockInfo,
          { useTransaction: true },
        );

        result.updatedEntities.push({ disabledKeys: keyIds });
      }
    }

    // TODO: Enable validation when we have support of non unique keys in DPP
    // add new
    // const withdrawPubKeyType = getWithdrawPubKeyTypeFromPayoutScript(newPayoutScript);
    // const pubKeyData = getPublicKeyFromPayoutScript(newPayoutScript, withdrawPubKeyType);

    // const newWithdrawalIdentityPublicKey = new IdentityPublicKey({
    //   id: identity.getPublicKeyMaxId() + 1,
    //   type: withdrawPubKeyType,
    //   data: pubKeyData,
    //   purpose: KeyPurpose.WITHDRAW,
    //   readOnly: true,
    //   securityLevel: KeySecurityLevel.MASTER,
    // });
    //
    // await identityPublicKeyRepository.add(
    //   identityId,
    //   [newWithdrawalIdentityPublicKey],
    //   blockInfo,
    //   { useTransaction: true },
    // );

    // result.createdEntities.push(newWithdrawalIdentityPublicKey);

    // identity.setRevision(identity.getRevision() + 1);

    // await identityRepository.updateRevision(
    //   identityId,
    //   identity.getRevision(),
    //   blockInfo,
    //   { useTransaction: true },
    // );

    result.updatedEntities.push(identity);

    return result;
  }

  return handleUpdatedScriptPayout;
}

module.exports = handleUpdatedScriptPayoutFactory;
