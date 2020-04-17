const bs58 = require('bs58');

const hash = require('../util/hash');

const Identity = require('./Identity');

const { decode } = require('../util/serializer');

const IdentityPublicKey = require('./IdentityPublicKey');
const IdentityCreateTransition = require('./stateTransitions/identityCreateTransition/IdentityCreateTransition');

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
   * @param {Buffer} lockedOutPoint
   * @param {PublicKey[]} [publicKeys]
   * @return {Identity}
   */
  create(lockedOutPoint, publicKeys = []) {
    const id = bs58.encode(
      hash(lockedOutPoint),
    );

    const identity = new Identity({
      id,
      balance: 0,
      publicKeys: publicKeys.map((publicKey, i) => ({
        id: i,
        type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
        data: publicKey.toBuffer()
          .toString('base64'),
        isEnabled: true,
      })),
    });

    identity.setLockedOutPoint(lockedOutPoint);

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

  /**
   * Create identity create transition
   *
   * @param {Identity} identity
   * @return {IdentityCreateTransition}
   */
  createIdentityCreateTransition(identity) {
    const lockedOutPoint = identity.getLockedOutPoint().toString('base64');

    const stateTransition = new IdentityCreateTransition({
      lockedOutPoint,
    });

    stateTransition.setPublicKeys(identity.getPublicKeys());

    return stateTransition;
  }
}

module.exports = IdentityFactory;
