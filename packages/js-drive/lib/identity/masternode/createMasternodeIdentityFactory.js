const IdentityPublicKey = require('@dashevo/dpp/lib/identity/IdentityPublicKey');
const Identity = require('@dashevo/dpp/lib/identity/Identity');

/**
 * @param {DashPlatformProtocol} dpp
 * @param {DriveStateRepository|CachedStateRepositoryDecorator} stateRepository
 * @return {createMasternodeIdentity}
 */
function createMasternodeIdentityFactory(dpp, stateRepository) {
  /**
   * @typedef createMasternodeIdentity
   * @param {Identifier} identityId
   * @param {Buffer} pubKeyData
   * @param {number} pubKeyType
   * @return {Promise<void>}
   */
  async function createMasternodeIdentity(identityId, pubKeyData, pubKeyType) {
    const identity = new Identity({
      protocolVersion: dpp.getProtocolVersion(),
      id: identityId,
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

    await stateRepository.storeIdentity(identity);

    const publicKeyHashes = identity
      .getPublicKeys()
      .map((publicKey) => publicKey.hash());

    await stateRepository.storeIdentityPublicKeyHashes(
      identity.getId(),
      publicKeyHashes,
    );
  }

  return createMasternodeIdentity;
}

module.exports = createMasternodeIdentityFactory;
