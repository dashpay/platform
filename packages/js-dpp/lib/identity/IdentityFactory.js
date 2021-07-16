const Identity = require('./Identity');

const { decode } = require('../util/serializer');

const IdentityPublicKey = require('./IdentityPublicKey');
const IdentityCreateTransition = require('./stateTransitions/identityCreateTransition/IdentityCreateTransition');
const IdentityTopUpTransition = require('./stateTransitions/identityTopUpTransition/IdentityTopUpTransition');

const InvalidIdentityError = require('./errors/InvalidIdentityError');
const SerializedObjectParsingError = require('../errors/SerializedObjectParsingError');
const InstantAssetLockProof = require('./stateTransitions/assetLockProof/instant/InstantAssetLockProof');
const ChainAssetLockProof = require('./stateTransitions/assetLockProof/chain/ChainAssetLockProof');

class IdentityFactory {
  /**
   * @param {validateIdentity} validateIdentity
   */
  constructor(validateIdentity) {
    this.validateIdentity = validateIdentity;
  }

  /**
   * Create Identity
   *
   * @param {InstantAssetLockProof} assetLockProof
   * @param {PublicKey[]} publicKeys
   * @return {Identity}
   */
  create(assetLockProof, publicKeys) {
    const identity = new Identity({
      protocolVersion: Identity.PROTOCOL_VERSION,
      id: assetLockProof.createIdentifier(),
      balance: 0,
      publicKeys: publicKeys.map((publicKey, i) => ({
        id: i,
        type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
        data: publicKey.toBuffer(),
      })),
      revision: 0,
    });

    identity.setAssetLockProof(assetLockProof);

    return identity;
  }

  /**
   * Create identity from a plain object
   *
   * @param {RawIdentity} rawIdentity
   * @param [options]
   * @param {boolean} [options.skipValidation]
   * @return {Identity}
   */
  createFromObject(rawIdentity, options = {}) {
    const opts = { skipValidation: false, ...options };

    if (!opts.skipValidation) {
      const result = this.validateIdentity(rawIdentity);

      if (!result.isValid()) {
        throw new InvalidIdentityError(result.getErrors(), rawIdentity);
      }
    }

    return new Identity(rawIdentity);
  }

  /**
   * Create Identity from a Buffer
   *
   * @param {Buffer} buffer
   * @param {Object} options
   * @param {boolean} [options.skipValidation=false]
   * @return {Identity}
   */
  createFromBuffer(buffer, options = {}) {
    let rawIdentity;
    try {
      // first 4 bytes are protocol version
      rawIdentity = decode(buffer.slice(4, buffer.length));
      rawIdentity.protocolVersion = buffer.slice(0, 4).readUInt32BE(0);
    } catch (error) {
      throw new InvalidIdentityError([
        new SerializedObjectParsingError(
          buffer,
          error,
        ),
      ]);
    }

    return this.createFromObject(rawIdentity, options);
  }

  /**
   * Create Instant Asset Lock proof
   *
   * @param {InstantLock} instantLock
   * @param {Transaction} assetLockTransaction
   * @param {number} outputIndex
   * @returns {InstantAssetLockProof}
   */
  createInstantAssetLockProof(instantLock, assetLockTransaction, outputIndex) {
    return new InstantAssetLockProof({
      instantLock: instantLock.toBuffer(),
      transaction: assetLockTransaction.toBuffer(),
      outputIndex,
    });
  }

  /**
   * Create Chain Asset Lock proof
   *
   * @param {number} coreChainLockedHeight
   * @param {Buffer} outPoint
   * @returns {InstantAssetLockProof}
   */
  createChainAssetLockProof(coreChainLockedHeight, outPoint) {
    return new ChainAssetLockProof({
      coreChainLockedHeight,
      outPoint,
    });
  }

  /**
   * Create identity create transition
   *
   * @param {Identity} identity
   * @return {IdentityCreateTransition}
   */
  createIdentityCreateTransition(identity) {
    const stateTransition = new IdentityCreateTransition({
      protocolVersion: Identity.PROTOCOL_VERSION,
      assetLockProof: identity.getAssetLockProof().toObject(),
    });

    stateTransition.setPublicKeys(identity.getPublicKeys());

    return stateTransition;
  }

  /**
   * Create identity top up transition
   *
   * @param {Identifier|Buffer|string} identityId - identity to top up
   * @param {InstantAssetLockProof} assetLockProof
   * @return {IdentityTopUpTransition}
   */
  createIdentityTopUpTransition(identityId, assetLockProof) {
    return new IdentityTopUpTransition({
      protocolVersion: Identity.PROTOCOL_VERSION,
      identityId,
      assetLockProof: assetLockProof.toObject(),
    });
  }
}

module.exports = IdentityFactory;
