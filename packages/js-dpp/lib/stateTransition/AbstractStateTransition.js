const {
  PublicKey,
  PrivateKey,
  Signer: { sign, verifySignature },
} = require('@dashevo/dashcore-lib');

const StateTransitionIsNotSignedError = require(
  './errors/StateTransitionIsNotSignedError',
);

const hash = require('../util/hash');
const { encode } = require('../util/serializer');

const calculateStateTransitionFee = require('./calculateStateTransitionFee');

/**
 * @abstract
 */
class AbstractStateTransition {
  /**
   * @param {
   * RawDataContractCreateTransition|
   * RawDocumentsBatchTransition|
   * RawIdentityCreateTransition|
   * RawIdentityTopUpTransition
   * } [rawStateTransition]
   */
  constructor(rawStateTransition = {}) {
    this.signature = null;
    this.protocolVersion = null;

    if (Object.prototype.hasOwnProperty.call(rawStateTransition, 'signature')) {
      this.signature = rawStateTransition.signature;
    }

    if (Object.prototype.hasOwnProperty.call(rawStateTransition, 'protocolVersion')) {
      this.protocolVersion = rawStateTransition.protocolVersion;
    }
  }

  /**
   * Get protocol version
   *
   * @return {number}
   */
  getProtocolVersion() {
    return this.protocolVersion;
  }

  /**
   * @abstract
   */
  getType() {
    throw new Error('Not implemented');
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
   * Set signature
   * @param {string} signature
   * @return {AbstractStateTransition}
   */
  setSignature(signature) {
    this.signature = signature;

    return this;
  }

  /**
   * Get state transition as plain object
   *
   * @param {Object} [options]
   * @param {boolean} [options.skipSignature]
   *
   * @return {Object}
   */
  toObject(options = {}) {
    const skipSignature = !!options.skipSignature;

    let plainObject = {
      protocolVersion: this.getProtocolVersion(),
      type: this.getType(),
    };

    if (!skipSignature) {
      plainObject = {
        ...plainObject,
        signature: this.getSignature(),
      };
    }

    return plainObject;
  }

  /**
   * Get state transition as JSON
   *
   * @param {Object} [options]
   *
   * @return {Object}
   */
  toJSON(options = {}) {
    return this.toObject({
      ...options,
    });
  }

  /**
   * Return serialized State Transition
   *
   * @param {Object} [options]
   * @return {Buffer}
   */
  serialize(options = {}) {
    return encode(this.toObject(options));
  }

  /**
   * Returns hex string with Data Contract hash
   *
   * @param {Object} [options]
   * @return {string}
   */
  hash(options = {}) {
    return hash(this.serialize(options)).toString('hex');
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

    const publicKeyModel = new PublicKey(publicKey, {});

    let isSignatureVerified;
    try {
      isSignatureVerified = verifySignature(data, signatureBuffer, publicKeyModel);
    } catch (e) {
      isSignatureVerified = false;
    }

    return isSignatureVerified;
  }

  /**
   * Calculate ST fee in credits
   *
   * @return {number}
   */
  calculateFee() {
    return calculateStateTransitionFee(this);
  }

  /**
   * @protected
   *
   * @param {Object} rawStateTransition
   *
   * @return {Object}
   */
  static translateJsonToObject(rawStateTransition) {
    return rawStateTransition;
  }
}

/**
 * @typedef RawStateTransition
 * @property {number} protocolVersion
 * @property {number} type
 * @property {string|null} signature
 */

module.exports = AbstractStateTransition;
