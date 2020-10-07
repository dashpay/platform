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
const EncodedBuffer = require('../util/encoding/EncodedBuffer');

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
    this.protocolVersion = rawStateTransition.protocolVersion;

    if (Object.prototype.hasOwnProperty.call(rawStateTransition, 'signature') && rawStateTransition.signature) {
      this.signature = EncodedBuffer.from(
        rawStateTransition.signature,
        EncodedBuffer.ENCODING.BASE64,
      );
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
   * @return {EncodedBuffer|null}
   */
  getSignature() {
    return this.signature;
  }

  /**
   * Set signature
   * @param {Buffer} signature
   * @return {AbstractStateTransition}
   */
  setSignature(signature) {
    this.signature = EncodedBuffer.from(signature, EncodedBuffer.ENCODING.BASE64);

    return this;
  }

  /**
   * Get state transition as plain object
   *
   * @param {Object} [options]
   * @param {boolean} [options.skipSignature=false]
   * @param {boolean} [options.encodedBuffer=false]
   *
   * @return {RawStateTransition}
   */
  toObject(options = {}) {
    Object.assign(
      options,
      {
        encodedBuffer: false,
        skipSignature: false,
        ...options,
      },
    );

    const rawStateTransition = {
      protocolVersion: this.getProtocolVersion(),
      type: this.getType(),
    };

    if (!options.skipSignature) {
      rawStateTransition.signature = this.getSignature();
    }

    if (!options.encodedBuffer && rawStateTransition.signature) {
      rawStateTransition.signature = this.getSignature().toBuffer();
    }

    return rawStateTransition;
  }

  /**
   * Get state transition as JSON
   *
   * @return {JsonStateTransition}
   */
  toJSON() {
    const jsonStateTransition = this.toObject({
      encodedBuffer: true,
    });

    if (jsonStateTransition.signature) {
      // noinspection JSValidateTypes
      jsonStateTransition.signature = jsonStateTransition.signature.toString();
    }

    // noinspection JSValidateTypes
    return jsonStateTransition;
  }

  /**
   * Return serialized State Transition
   *
   * @param {Object} [options]
   * @param {boolean} [options.skipSignature=false]
   * @return {Buffer}
   */
  toBuffer(options = {}) {
    return encode(this.toObject(options));
  }

  /**
   * Returns hex string with Data Contract hash
   *
   * @param {Object} [options]
   * @return {string}
   */
  hash(options = {}) {
    return hash(this.toBuffer(options)).toString('hex');
  }

  /**
   * Sign data with private key
   * @param {string|Buffer|Uint8Array|PrivateKey} privateKey string must be hex or base58
   * @return {AbstractStateTransition}
   */
  signByPrivateKey(privateKey) {
    const data = this.toBuffer({ skipSignature: true });
    const privateKeyModel = new PrivateKey(privateKey);

    this.setSignature(sign(data, privateKeyModel));

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

    const signatureBuffer = signature.toBuffer();
    const data = this.toBuffer({ skipSignature: true });

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
}

/**
 * @typedef RawStateTransition
 * @property {number} protocolVersion
 * @property {number} type
 * @property {Buffer} [signature]
 */

/**
 * @typedef JsonStateTransition
 * @property {number} protocolVersion
 * @property {number} type
 * @property {string} [signature]
 */

AbstractStateTransition.ENCODED_PROPERTIES = {
  signature: {
    contentEncoding: 'base64',
  },
};

module.exports = AbstractStateTransition;
