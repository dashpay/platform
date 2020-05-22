const IdentityFactory = require('./IdentityFactory');
const validateIdentityFactory = require('./validation/validateIdentityFactory');
const validatePublicKeysFactory = require('./validation/validatePublicKeysFactory');

/**
 * @class IdentityFacade
 * @property {validateIdentity} validateIdentity
 */
class IdentityFacade {
  /**
   * @param {JsonSchemaValidator} validator
   */
  constructor(validator) {
    const validatePublicKeys = validatePublicKeysFactory(
      validator,
    );
    this.validateIdentity = validateIdentityFactory(
      validator,
      validatePublicKeys,
    );
    this.factory = new IdentityFactory(this.validateIdentity);
  }

  /**
   * Create Identity
   *
   * @param {Buffer} lockedOutPoint
   * @param {PublicKey[]} [publicKeys]
   * @return {Identity}
   */
  create(lockedOutPoint, publicKeys = []) {
    return this.factory.create(lockedOutPoint, publicKeys);
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
   * Create identity from a string/Buffer
   *
   * @param {Buffer|string} serializedIdentity
   * @param [options]
   * @param {boolean} [options.skipValidation]
   * @return {Identity}
   */
  createFromSerialized(serializedIdentity, options = {}) {
    return this.factory.createFromSerialized(serializedIdentity, options);
  }

  /**
   * Validate identity
   *
   * @param {Identity|RawIdentity} identity
   * @return {ValidationResult}
   */
  validate(identity) {
    return this.validateIdentity(identity);
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
   * @param {string} identityId - identity id to top up
   * @param {Buffer} lockedOutPointBuffer - outpoint of the top up output of the L1 transaction
   * @return {IdentityTopUpTransition}
   */
  createIdentityTopUpTransition(identityId, lockedOutPointBuffer) {
    return this.factory.createIdentityTopUpTransition(identityId, lockedOutPointBuffer);
  }
}

module.exports = IdentityFacade;
