const Identity = require('./Identity');

const { decode } = require('../util/serializer');

const IdentityPublicKey = require('./IdentityPublicKey');
const IdentityCreateTransition = require('./stateTransitions/identityCreateTransition/IdentityCreateTransition');
const IdentityTopUpTransition = require('./stateTransitions/identityTopUpTransition/IdentityTopUpTransition');

const InvalidIdentityError = require('./errors/InvalidIdentityError');
const SerializedObjectParsingError = require('../errors/SerializedObjectParsingError');
const AssetLock = require('./stateTransitions/assetLock/AssetLock');
const InstantAssetLockProof = require('./stateTransitions/assetLock/proof/instant/InstantAssetLockProof');

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
   * @param {Transaction} assetLockTransaction
   * @param {number} outputIndex
   * @param {InstantAssetLockProof} assetLockProof
   * @param {PublicKey[]} publicKeys
   * @return {Identity}
   */
  create(assetLockTransaction, outputIndex, assetLockProof, publicKeys) {
    const assetLock = new AssetLock({
      transaction: assetLockTransaction.toBuffer(),
      outputIndex,
      proof: assetLockProof.toObject(),
    });

    const identity = new Identity({
      protocolVersion: Identity.PROTOCOL_VERSION,
      id: assetLock.createIdentifier(),
      balance: 0,
      publicKeys: publicKeys.map((publicKey, i) => ({
        id: i,
        type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
        data: publicKey.toBuffer(),
      })),
      revision: 0,
    });

    identity.setAssetLock(assetLock);

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
      rawIdentity = decode(buffer);
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
   * Create Asset Lock with proofs
   *
   * @param {InstantLock} instantLock
   * @returns {InstantAssetLockProof}
   */
  createInstantAssetLockProof(instantLock) {
    return new InstantAssetLockProof({
      instantLock: instantLock.toBuffer(),
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
      assetLock: identity.getAssetLock().toObject(),
    });

    stateTransition.setPublicKeys(identity.getPublicKeys());

    return stateTransition;
  }

  /**
   * Create identity top up transition
   *
   * @param {Identifier|Buffer|string} identityId - identity to top up
   * @param {Transaction} assetLockTransaction
   * @param {number} outputIndex
   * @param {InstantAssetLockProof} assetLockProof
   * @return {IdentityTopUpTransition}
   */
  createIdentityTopUpTransition(identityId, assetLockTransaction, outputIndex, assetLockProof) {
    const assetLock = new AssetLock({
      transaction: assetLockTransaction.toBuffer(),
      outputIndex,
      proof: assetLockProof.toObject(),
    });

    return new IdentityTopUpTransition({
      protocolVersion: Identity.PROTOCOL_VERSION,
      identityId,
      assetLock: assetLock.toObject(),
    });
  }
}

module.exports = IdentityFactory;
