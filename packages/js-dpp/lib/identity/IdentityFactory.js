const Identity = require('./Identity');

const { decode } = require('../util/serializer');

const InvalidIdentityError = require('./errors/InvalidIdentityError');
const SerializedObjectParsingError = require('../errors/SerializedObjectParsingError');

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
   * @param {string} id
   * @param {number} type
   * @param {IdentityPublicKey[]} [publicKeys]
   * @return {Identity}
   */
  create(id, type, publicKeys = []) {
    return new Identity({
      id,
      type,
      publicKeys: publicKeys.map((p) => p.toJSON()),
    });
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
   * Create Identity from a string/Buffer
   *
   * @param {Buffer|string} serializedIdentity
   * @param {Object} options
   * @param {boolean} [options.skipValidation=false]
   * @return {Identity}
   */
  createFromSerialized(serializedIdentity, options = {}) {
    let rawIdentity;
    try {
      rawIdentity = decode(serializedIdentity);
    } catch (error) {
      throw new InvalidIdentityError([
        new SerializedObjectParsingError(
          serializedIdentity,
          error,
        ),
      ]);
    }

    return this.createFromObject(rawIdentity, options);
  }
}

module.exports = IdentityFactory;
