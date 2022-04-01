const IdentityPublicKey = require('@dashevo/dpp/lib/identity/IdentityPublicKey');
const Identity = require('@dashevo/dpp/lib/identity/Identity');
const Script = require('@dashevo/dashcore-lib/lib/script');
const InvalidMasternodeIdentityError = require('./errors/InvalidMasternodeIdentityError');

/**
 * @param {DashPlatformProtocol} dpp
 * @param {DriveStateRepository|CachedStateRepositoryDecorator} transactionalStateRepository
 * @param {string} network
 * @return {createMasternodeIdentity}
 */
function createMasternodeIdentityFactory(
  dpp,
  transactionalStateRepository,
  network,
) {
  /**
   * @typedef createMasternodeIdentity
   * @param {Identifier} identifier
   * @param {Buffer} pubKeyData
   * @param {number} pubKeyType
   * @param {Buffer} withdrawPubKeyData
   * @return {Promise<void>}
   */
  async function createMasternodeIdentity(
    identifier,
    pubKeyData,
    pubKeyType,
    withdrawPubKeyData,
  ) {
    const script = new Script(withdrawPubKeyData);
    const withdrawPubKeyType = script.toAddress(network).isPayToScriptHash()
      ? IdentityPublicKey.TYPES.BIP13_SCRIPT_HASH
      : IdentityPublicKey.TYPES.ECDSA_HASH160;

    const identity = new Identity({
      protocolVersion: dpp.getProtocolVersion(),
      id: identifier.toBuffer(),
      publicKeys: [{
        id: 0,
        type: pubKeyType,
        purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
        securityLevel: IdentityPublicKey.SECURITY_LEVELS.MASTER,
        readOnly: true,
        // Copy data buffer
        data: Buffer.from(pubKeyData),
      }, {
        id: 1,
        type: withdrawPubKeyType,
        purpose: IdentityPublicKey.PURPOSES.WITHDRAW,
        securityLevel: IdentityPublicKey.SECURITY_LEVELS.MASTER,
        readonly: false,
        data: Buffer.from(withdrawPubKeyData),
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

    const publicKeyHashes = identity
      .getPublicKeys()
      .map((publicKey) => publicKey.hash());

    await transactionalStateRepository.storeIdentityPublicKeyHashes(
      identity.getId(),
      publicKeyHashes,
    );
  }

  return createMasternodeIdentity;
}

module.exports = createMasternodeIdentityFactory;
