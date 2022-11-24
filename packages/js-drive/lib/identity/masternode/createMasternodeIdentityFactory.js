const IdentityPublicKey = require('@dashevo/dpp/lib/identity/IdentityPublicKey');
const Identity = require('@dashevo/dpp/lib/identity/Identity');
const InvalidMasternodeIdentityError = require('./errors/InvalidMasternodeIdentityError');

/**
 * @param {DashPlatformProtocol} dpp
 * @param {IdentityStoreRepository} identityRepository
 * @param {PublicKeyToIdentitiesStoreRepository} publicKeyToIdentitiesRepository
 * @param {getWithdrawPubKeyTypeFromPayoutScript} getWithdrawPubKeyTypeFromPayoutScript
 * @param {getPublicKeyFromPayoutScript} getPublicKeyFromPayoutScript
 * @return {createMasternodeIdentity}
 */
function createMasternodeIdentityFactory(
  dpp,
  identityRepository,
  publicKeyToIdentitiesRepository,
  getWithdrawPubKeyTypeFromPayoutScript,
  getPublicKeyFromPayoutScript,
) {
  /**
   * @typedef createMasternodeIdentity
   * @param {Identifier} identifier
   * @param {Buffer} pubKeyData
   * @param {number} pubKeyType
   * @param {GroveDBTransaction} transaction
   * @param {Script} [payoutScript]
   * @return {Promise<Identity>}
   */
  async function createMasternodeIdentity(
    identifier,
    pubKeyData,
    pubKeyType,
    transaction,
    payoutScript,
  ) {
    const publicKeys = [{
      id: 0,
      type: pubKeyType,
      purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
      securityLevel: IdentityPublicKey.SECURITY_LEVELS.MASTER,
      readOnly: true,
      // Copy data buffer
      data: Buffer.from(pubKeyData),
    }];

    if (payoutScript) {
      const withdrawPubKeyType = getWithdrawPubKeyTypeFromPayoutScript(payoutScript);

      publicKeys.push({
        id: 1,
        type: withdrawPubKeyType,
        purpose: IdentityPublicKey.PURPOSES.WITHDRAW,
        securityLevel: IdentityPublicKey.SECURITY_LEVELS.CRITICAL,
        readOnly: false,
        data: getPublicKeyFromPayoutScript(payoutScript, withdrawPubKeyType),
      });
    }

    const identity = new Identity({
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

    await identityRepository.create(identity, {
      transaction,
    });

    const publicKeyHashes = identity
      .getPublicKeys()
      .map((publicKey) => publicKey.hash());

    for (const hash of publicKeyHashes) {
      await publicKeyToIdentitiesRepository.store(
        hash,
        identity.getId(),
        { transaction },
      );
    }

    return identity;
  }

  return createMasternodeIdentity;
}

module.exports = createMasternodeIdentityFactory;
