const IdentityPublicKey = require('@dashevo/dpp/lib/identity/IdentityPublicKey');
const Identity = require('@dashevo/dpp/lib/identity/Identity');

/**
 * @param {DashPlatformProtocol} dpp
 * @param {DriveStateRepository|CachedStateRepositoryDecorator} stateRepository
 * @param {IdentityStoreRepository} previousIdentityRepository
 * @param {PublicKeyToIdentityIdStoreRepository} previousPublicKeyToIdentityIdRepository
 * @return {createMasternodeIdentity}
 */
function createMasternodeIdentityFactory(
  dpp,
  stateRepository,
  previousIdentityRepository,
  previousPublicKeyToIdentityIdRepository,
) {
  /**
   * @typedef createMasternodeIdentity
   * @param {Identifier} identityId
   * @param {Buffer} pubKeyData
   * @param {number} pubKeyType
   * @param {boolean} storePreviousState
   * @return {Promise<void>}
   */
  async function createMasternodeIdentity(
    identityId,
    pubKeyData,
    pubKeyType,
    storePreviousState,
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

    await stateRepository.storeIdentity(identity);

    if (storePreviousState) {
      await previousIdentityRepository.store(identity);
    }

    const publicKeyHashes = identity
      .getPublicKeys()
      .map((publicKey) => publicKey.hash());

    await stateRepository.storeIdentityPublicKeyHashes(
      identity.getId(),
      publicKeyHashes,
    );

    if (storePreviousState) {
      for (const publicKeyHash of publicKeyHashes) {
        await previousPublicKeyToIdentityIdRepository
          .store(
            publicKeyHash, identityId,
          );
      }
    }
  }

  return createMasternodeIdentity;
}

module.exports = createMasternodeIdentityFactory;
