const InvalidMasternodeIdentityError = require('./errors/InvalidMasternodeIdentityError');

/**
 * @param {DashPlatformProtocol} dpp
 * @param {IdentityStoreRepository} identityRepository
 * @param {getWithdrawPubKeyTypeFromPayoutScript} getWithdrawPubKeyTypeFromPayoutScript
 * @param {getPublicKeyFromPayoutScript} getPublicKeyFromPayoutScript
 * @param {WebAssembly.Instance} dppWasm
 * @return {createMasternodeIdentity}
 */
function createMasternodeIdentityFactory(
  dpp,
  identityRepository,
  //getWithdrawPubKeyTypeFromPayoutScript,
  //getPublicKeyFromPayoutScript,
  dppWasm,
) {
  /**
   * @typedef createMasternodeIdentity
   * @param {BlockInfo} blockInfo
   * @param {Identifier} identifier
   * @param {Buffer} pubKeyData
   * @param {number} pubKeyType
   * @param {Script} [payoutScript]
   * @return {Promise<Identity>}
   */
  async function createMasternodeIdentity(
    blockInfo,
    identifier,
    pubKeyData,
    pubKeyType,
    // payoutScript,
  ) {
    const publicKeys = [{
      id: 0,
      type: pubKeyType,
      purpose: dppWasm.KeyPurpose.AUTHENTICATION,
      securityLevel: dppWasm.KeySecurityLevel.MASTER,
      readOnly: true,
      // Copy data buffer
      data: Buffer.from(pubKeyData),
    }];

    // TODO: Enable keys when we have support of non unique keys in DPP
    // if (payoutScript) {
    //   const withdrawPubKeyType = getWithdrawPubKeyTypeFromPayoutScript(payoutScript);
    //
    //   publicKeys.push({
    //     id: 1,
    //     type: withdrawPubKeyType,
    //     purpose: IdentityPublicKey.PURPOSES.WITHDRAW,
    //     securityLevel: IdentityPublicKey.SECURITY_LEVELS.CRITICAL,
    //     readOnly: false,
    //     data: getPublicKeyFromPayoutScript(payoutScript, withdrawPubKeyType),
    //   });
    // }

    const identity = new dppWasm.Identity({
      protocolVersion: dpp.getProtocolVersion(),
      id: identifier.toBuffer(),
      publicKeys,
      balance: 0,
      revision: 0,
    });

    const validationResult = await dpp.identity.validate(identity);
    if (!validationResult.isValid()) {
      const validationError = validationResult.getFirstError();

      throw new InvalidMasternodeIdentityError(validationError);
    }

    await identityRepository.create(identity, blockInfo, {
      useTransaction: true,
    });

    return identity;
  }

  return createMasternodeIdentity;
}

module.exports = createMasternodeIdentityFactory;
