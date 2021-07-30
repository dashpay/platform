const Identity = require('./Identity');
const IdentityFactory = require('./IdentityFactory');

const validateIdentityFactory = require('./validation/validateIdentityFactory');
const validatePublicKeysFactory = require('./validation/validatePublicKeysFactory');

/**
 * @class IdentityFacade
 * @property {validateIdentity} validateIdentity
 */
class IdentityFacade {
  /**
   * @param {DashPlatformProtocol} dpp
   */
  constructor(dpp) {
    const validatePublicKeys = validatePublicKeysFactory(
      dpp.getJsonSchemaValidator(),
    );
    this.validateIdentity = validateIdentityFactory(
      dpp.getJsonSchemaValidator(),
      validatePublicKeys,
    );
    this.factory = new IdentityFactory(dpp, this.validateIdentity);
  }

  /**
   * Create Identity
   *
   * @param {InstantAssetLockProof|ChainAssetLockProof} assetLockProof
   * @param {PublicKey[]} publicKeys
   * @return {Identity}
   */
  create(assetLockProof, publicKeys) {
    return this.factory.create(
      assetLockProof,
      publicKeys,
    );
  }

  /**
   * Create Identity from the plain object
   *
   * @param {RawIdentity} rawIdentity
   * @param [options]
   * @param {boolean} [options.skipValidation]
   * @return {Identity}
   */
  createFromObject(rawIdentity, options = {}) {
    return this.factory.createFromObject(rawIdentity, options);
  }

  /**
   * Create identity from a Buffer
   *
   * @param {Buffer} buffer
   * @param [options]
   * @param {boolean} [options.skipValidation]
   * @return {Identity}
   */
  createFromBuffer(buffer, options = {}) {
    return this.factory.createFromBuffer(buffer, options);
  }

  /**
   * Validate identity
   *
   * @param {Identity|RawIdentity} identity
   * @return {ValidationResult}
   */
  validate(identity) {
    let rawIdentity;
    if (identity instanceof Identity) {
      rawIdentity = identity.toObject();
    } else {
      rawIdentity = identity;
    }

    return this.validateIdentity(rawIdentity);
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
    return this.factory.createInstantAssetLockProof(instantLock, assetLockTransaction, outputIndex);
  }

  /**
   * Create Chain Asset Lock proof
   *
   * @param {number} coreChainLockedHeight
   * @param {Buffer} outPoint
   * @returns {InstantAssetLockProof|ChainAssetLockProof}
   */
  createChainAssetLockProof(coreChainLockedHeight, outPoint) {
    return this.factory.createChainAssetLockProof(coreChainLockedHeight, outPoint);
  }

  /**
   * Create identity create transition
   *
   * @param {Identity} identity
   * @return {IdentityCreateTransition}
   */
  createIdentityCreateTransition(identity) {
    return this.factory.createIdentityCreateTransition(identity);
  }

  /**
   * Create identity top up transition
   *
   * @param {Identifier|Buffer|string} identityId - identity to top up
   * @param {InstantAssetLockProof|ChainAssetLockProof} assetLockProof
   * @return {IdentityTopUpTransition}
   */
  createIdentityTopUpTransition(identityId, assetLockProof) {
    return this.factory.createIdentityTopUpTransition(
      identityId,
      assetLockProof,
    );
  }
}

module.exports = IdentityFacade;
