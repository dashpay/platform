const Identity = require('./Identity');
const IdentityFactory = require('./IdentityFactory');

const validateIdentityFactory = require('./validation/validateIdentityFactory');
const validatePublicKeysFactory = require('./validation/validatePublicKeysFactory');
const decodeProtocolEntityFactory = require('../decodeProtocolEntityFactory');

const publicKeyJsonSchema = require('../../schema/identity/publicKey.json');

const protocolVersion = require('../version/protocolVersion');
const validateProtocolVersionFactory = require('../version/validateProtocolVersionFactory');

/**
 * @class IdentityFacade
 * @property {validateIdentity} validateIdentity
 */
class IdentityFacade {
  /**
   * @param {DashPlatformProtocol} dpp
   * @param {BlsSignatures} bls
   */
  constructor(dpp, bls) {
    const validatePublicKeys = validatePublicKeysFactory(
      dpp.getJsonSchemaValidator(),
      publicKeyJsonSchema,
      bls,
    );

    const validateProtocolVersion = validateProtocolVersionFactory(
      dpp,
      protocolVersion.compatibility,
    );

    this.validateIdentity = validateIdentityFactory(
      dpp.getJsonSchemaValidator(),
      validatePublicKeys,
      validateProtocolVersion,
    );

    const decodeProtocolEntity = decodeProtocolEntityFactory();

    this.factory = new IdentityFactory(
      dpp,
      this.validateIdentity,
      decodeProtocolEntity,
    );
  }

  /**
   * Create Identity
   *
   * @param {InstantAssetLockProof|ChainAssetLockProof} assetLockProof
   * @param {PublicKeyConfig[]} publicKeyConfigs
   * @return {Identity}
   */
  create(assetLockProof, publicKeyConfigs) {
    return this.factory.create(
      assetLockProof,
      publicKeyConfigs,
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

  /**
   * Create identity update transition
   *
   * @param {Identity} identity
   * @param {{add: IdentityPublicKey[]; disable: IdentityPublicKey[]}} publicKeys
   * @returns {IdentityUpdateTransition}
   */
  createIdentityUpdateTransition(identity, publicKeys) {
    return this.factory.createIdentityUpdateTransition(identity, publicKeys);
  }
}

module.exports = IdentityFacade;
