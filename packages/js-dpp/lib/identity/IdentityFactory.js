const bs58 = require('bs58');

const hash = require('../util/hash');

const Identity = require('./Identity');

const { decode } = require('../util/serializer');

const IdentityPublicKey = require('./IdentityPublicKey');
const IdentityCreateTransition = require('./stateTransitions/identityCreateTransition/IdentityCreateTransition');
const IdentityTopUpTransition = require('./stateTransitions/identityTopUpTransition/IdentityTopUpTransition');

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
      protocolVersion: Identity.PROTOCOL_VERSION,
      id,
      balance: 0,
      publicKeys: publicKeys.map((publicKey, i) => ({
        id: i,
        type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
        data: publicKey.toBuffer()
          .toString('base64'),
      })),
      revision: 0,
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

    const identity = new Identity(rawIdentity);

    if (!opts.skipValidation) {
      const result = this.validateIdentity(identity.toJSON());

      if (!result.isValid()) {
        throw new InvalidIdentityError(result.getErrors(), rawIdentity);
      }
    }

    return identity;
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
      protocolVersion: Identity.PROTOCOL_VERSION,
      lockedOutPoint,
    });

    stateTransition.setPublicKeys(identity.getPublicKeys());

    return stateTransition;
  }

  /**
   * Create identity top up transition
   *
   * @param {string} identityId - identity to top up, base58 encoded
   * @param {Buffer} lockedOutPointBuffer - pointer to outpoint of funding transaction
   * @return {IdentityTopUpTransition}
   */
  createIdentityTopUpTransition(identityId, lockedOutPointBuffer) {
    const lockedOutPoint = lockedOutPointBuffer.toString('base64');

    return new IdentityTopUpTransition({
      protocolVersion: Identity.PROTOCOL_VERSION,
      identityId,
      lockedOutPoint,
    });
  }
}

module.exports = IdentityFactory;
