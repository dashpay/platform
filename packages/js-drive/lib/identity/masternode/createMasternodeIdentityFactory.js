const IdentityPublicKey = require('@dashevo/dpp/lib/identity/IdentityPublicKey');
const Identity = require('@dashevo/dpp/lib/identity/Identity');
const InvalidMasternodeIdentityError = require('./errors/InvalidMasternodeIdentityError');

/**
 * @param {DashPlatformProtocol} dpp
 * @param {DriveStateRepository|CachedStateRepositoryDecorator} transactionalStateRepository
 * @return {createMasternodeIdentity}
 */
function createMasternodeIdentityFactory(
  dpp,
  transactionalStateRepository,
) {
  /**
   * @typedef createMasternodeIdentity
   * @param {Identifier} identityId
   * @param {Buffer} pubKeyData
   * @param {number} pubKeyType
   * @return {Promise<void>}
   */
  async function createMasternodeIdentity(
    identityId,
    pubKeyData,
    pubKeyType,
  ) {
    const identity = new Identity({
      protocolVersion: dpp.getProtocolVersion(),
      id: identityId.toBuffer(),
      publicKeys: [{
        id: 0,
        type: pubKeyType,
        purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
        securityLevel: IdentityPublicKey.SECURITY_LEVELS.MASTER,
        readOnly: true,
        // Copy data buffer
        data: Buffer.from(pubKeyData),
      }],
      balance: 0,
      revision: 0,
    });

    const validationResult = await dpp.identity.validate(identity);
    if (!validationResult.isValid()) {
      const validationError = validationResult.getFirstError();

      throw new InvalidMasternodeIdentityError(validationError);
    }

    await transactionalStateRepository.storeIdentity(identity);

    const publicKeyHashes = await Promise.all(
      identity
        .getPublicKeys()
        .map(async (publicKey) => publicKey.hash()),
    );

    await transactionalStateRepository.storeIdentityPublicKeyHashes(
      identity.getId(),
      publicKeyHashes,
    );
  }

  return createMasternodeIdentity;
}

module.exports = createMasternodeIdentityFactory;
