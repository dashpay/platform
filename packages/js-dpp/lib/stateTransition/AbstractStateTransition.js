const {
  PublicKey,
  PrivateKey,
  Signer: { sign, verifySignature },
} = require('@dashevo/dashcore-lib');

const hash = require('../util/hash');
const { encode } = require('../util/serializer');
const IdentityPublicKey = require('../identity/IdentityPublicKey');
const InvalidSignatureTypeError = require('./errors/InvalidSignatureTypeError');
const InvalidSignaturePublicKeyError = require('./errors/InvalidSignaturePublicKeyError');
const StateTransitionIsNotSignedError = require('./errors/StateTransitionIsNotSignedError');
const PublicKeyMismatchError = require('./errors/PublicKeyMismatchError');

/**
 * @abstract
 */
class AbstractStateTransition {
  constructor() {
    this.signaturePublicKeyId = null;
    this.signature = null;
  }

  /**
   * Get protocol version
   *
   * @return {number}
   */
  getProtocolVersion() {
    return 0;
  }

  /**
   * @abstract
   */
  getType() {
    throw new Error('Not implemented');
  }

  /**
   * @abstract
   * @param {Object} [options]
   * @return {{protocolVersion: number, type: number, [sign]: string, [keyId]: number}}
   */
  toJSON(options = {}) {
    const skipSignature = !!options.skipSignature;

    let json = {
      protocolVersion: this.getProtocolVersion(),
      type: this.getType(),
    };

    if (!skipSignature) {
      json = {
        ...json,
        signature: this.getSignature(),
        signaturePublicKeyId: this.getSignaturePublicKeyId(),
      };
    }

    return json;
  }

  /**
   * Return serialized State Transition
   *
   * @param {Object} [options]
   * @return {Buffer}
   */
  serialize(options = {}) {
    return encode(this.toJSON(options));
  }

  /**
   * Returns hex string with Data Contract hash
   *
   * @return {string}
   */
  hash() {
    return hash(this.serialize()).toString('hex');
  }

  /**
   *  Returns signature
   *
   * @return {string|null}
   */
  getSignature() {
    return this.signature;
  }

  /**
   * Returns public key id
   *
   * @returns {number|null}
   */
  getSignaturePublicKeyId() {
    return this.signaturePublicKeyId;
  }

  /**
   * Sign data and check identityPublicKey
   *
   * @param {IdentityPublicKey} identityPublicKey
   * @param {string|Buffer|Uint8Array|PrivateKey} privateKey string must be hex or base58
   * @return {AbstractStateTransition}
   */
  sign(identityPublicKey, privateKey) {
    const data = this.serialize({ skipSignature: true });
    let privateKeyModel;
    let pubKeyBase;

    switch (identityPublicKey.getType()) {
      case IdentityPublicKey.TYPES.ECDSA_SECP256K1:
        privateKeyModel = new PrivateKey(privateKey);

        /* We store compressed public key in the identity as a base64 string,
        /* and here we compare the private key used to sign the state transition
        /* with the compressed key stored in the identity */
        pubKeyBase = new PublicKey({
          ...privateKeyModel.toPublicKey().toObject(),
          compressed: true,
        })
          .toBuffer()
          .toString('base64');

        if (pubKeyBase !== identityPublicKey.getData()) {
          throw new InvalidSignaturePublicKeyError(identityPublicKey.getData());
        }

        this.signature = sign(data, privateKeyModel).toString('base64');

        break;
      default:
        throw new InvalidSignatureTypeError(identityPublicKey.getType());
    }

    this.signaturePublicKeyId = identityPublicKey.getId();

    return this;
  }

  /**
   * Sign data with private key
   * @param {string|Buffer|Uint8Array|PrivateKey} privateKey string must be hex or base58
   * @return {AbstractStateTransition}
   */
  signByPrivateKey(privateKey) {
    const data = this.serialize({ skipSignature: true });
    const privateKeyModel = new PrivateKey(privateKey);

    this.signature = sign(data, privateKeyModel).toString('base64');

    return this;
  }

  /**
   * Verify signature
   *
   * @param {IdentityPublicKey} publicKey
   * @return {boolean}
   */
  verifySignature(publicKey) {
    const signature = this.getSignature();
    if (!signature) {
      throw new StateTransitionIsNotSignedError(this);
    }

    if (this.getSignaturePublicKeyId() !== publicKey.getId()) {
      throw new PublicKeyMismatchError(publicKey);
    }

    const signatureBuffer = Buffer.from(signature, 'base64');

    const data = this.serialize({ skipSignature: true });

    const publicKeyBuffer = Buffer.from(publicKey.getData(), 'base64');
    const publicKeyModel = PublicKey.fromBuffer(publicKeyBuffer);

    let isSignatureVerified;
    try {
      isSignatureVerified = verifySignature(data, signatureBuffer, publicKeyModel);
    } catch (e) {
      isSignatureVerified = false;
    }

    return isSignatureVerified;
  }

  /**
   * Verify signature with public key
   * @param {string|Buffer|Uint8Array|PublicKey} publicKey string must be hex or base58
   * @returns {boolean}
   */
  verifySignatureByPublicKey(publicKey) {
    const signature = this.getSignature();
    if (!signature) {
      throw new StateTransitionIsNotSignedError(this);
    }

    const signatureBuffer = Buffer.from(signature, 'base64');
    const data = this.serialize({ skipSignature: true });

    const publicKeyModel = new PublicKey(publicKey);

    let isSignatureVerified;
    try {
      isSignatureVerified = verifySignature(data, signatureBuffer, publicKeyModel);
    } catch (e) {
      isSignatureVerified = false;
    }

    return isSignatureVerified;
  }

  /**
   * Set signature
   * @param {string|null} [signature]
   * @return {AbstractStateTransition}
   */
  setSignature(signature = null) {
    this.signature = signature;

    return this;
  }

  /**
   * Set signature public key id
   * @param {number|null} [signaturePublicKeyId]
   * @return {AbstractStateTransition}
   */
  setSignaturePublicKeyId(signaturePublicKeyId = null) {
    this.signaturePublicKeyId = signaturePublicKeyId;

    return this;
  }
}

module.exports = AbstractStateTransition;
