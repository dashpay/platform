const IdentityFactory = require('./IdentityFactory');
const validateIdentityFactory = require('./validation/validateIdentityFactory');
const applyIdentityStateTransition = require('./stateTransitions/applyIdentityStateTransition');
const validateIdentityType = require('./validation/validateIdentityType');
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
      validateIdentityType,
      validatePublicKeys,
    );
    this.factory = new IdentityFactory(this.validateIdentity);
    this.applyIdentityStateTransition = applyIdentityStateTransition;
  }

  /**
   * Create Identity
   *
   * @param {string} id
   * @param {number} type
   * @param {IdentityPublicKey[]} [publicKeys]
   * @return {Identity}
   */
  create(id, type, publicKeys = []) {
    return this.factory.create(id, type, publicKeys);
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
   * Applies a state transition to the identity model
   *
   * @param {IdentityCreateTransition} stateTransition
   * @param {Identity|null} identity
   * @return {Identity|null}
   */
  applyStateTransition(stateTransition, identity) {
    return this.applyIdentityStateTransition(stateTransition, identity);
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
}

module.exports = IdentityFacade;
