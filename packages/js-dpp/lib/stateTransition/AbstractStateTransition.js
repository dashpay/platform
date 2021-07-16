const {
  PublicKey,
  PrivateKey,
  Signer: { sign, verifySignature },
} = require('@dashevo/dashcore-lib');

const StateTransitionIsNotSignedError = require(
  './errors/StateTransitionIsNotSignedError',
);

const stateTransitionTypes = require('./stateTransitionTypes');

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
    this.protocolVersion = rawStateTransition.protocolVersion;

    if (Object.prototype.hasOwnProperty.call(rawStateTransition, 'signature')) {
      this.signature = rawStateTransition.signature;
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
   *
   * @return {number}
   */
  getType() {
    throw new Error('Not implemented');
  }

  /**
   *  Returns signature
   *
   * @return {Buffer}
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
    this.signature = signature;

    return this;
  }

  /**
   * Get state transition as plain object
   *
   * @param {Object} [options]
   * @param {boolean} [options.skipSignature=false]
   * @param {boolean} [options.skipIdentifiersConversion=false]
   *
   * @return {RawStateTransition}
   */
  toObject(options = {}) {
    Object.assign(
      options,
      {
        skipIdentifiersConversion: false,
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

    return rawStateTransition;
  }

  /**
   * Get state transition as JSON
   *
   * @return {JsonStateTransition}
   */
  toJSON() {
    const jsonStateTransition = this.toObject({ skipIdentifiersConversion: true });

    if (jsonStateTransition.signature) {
      // noinspection JSValidateTypes
      jsonStateTransition.signature = jsonStateTransition.signature.toString('base64');
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
    const serializedData = this.toObject(options);
    delete serializedData.protocolVersion;

    const protocolVersionUInt32 = Buffer.alloc(4);
    protocolVersionUInt32.writeUInt32BE(this.getProtocolVersion(), 0);

    return Buffer.concat([protocolVersionUInt32, encode(serializedData)]);
  }

  /**
   * Returns hex string with Data Contract hash
   *
   * @param {Object} [options]
   * @param {boolean} [options.skipSignature=false]
   * @return {Buffer}
   */
  hash(options = {}) {
    return hash(this.toBuffer(options));
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

    const data = this.toBuffer({ skipSignature: true });

    const publicKeyModel = new PublicKey(publicKey, {});

    let isSignatureVerified;
    try {
      isSignatureVerified = verifySignature(data, signature, publicKeyModel);
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
   * Returns ids of entities affected by the state transition
   * @abstract
   *
   * @return {Identifier[]}
   */
  getModifiedDataIds() {
    throw new Error('Not implemented');
  }

  /**
   * Returns true if this state transition affects documents: create, update and delete transitions
   *
   * @return {boolean}
   */
  isDocumentStateTransition() {
    return AbstractStateTransition.documentTransitionTypes.includes(this.getType());
  }

  /**
   * Returns true if this state transition affects data contracts
   *
   * @return {boolean}
   */
  isDataContractStateTransition() {
    return AbstractStateTransition.dataContractTransitionTypes.includes(this.getType());
  }

  /**
   * Returns true if this state transition affects identities: create, update or top up.
   *
   * @return {boolean}
   */
  isIdentityStateTransition() {
    return AbstractStateTransition.identityTransitionTypes.includes(this.getType());
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

AbstractStateTransition.documentTransitionTypes = [
  stateTransitionTypes.DOCUMENTS_BATCH,
];
AbstractStateTransition.identityTransitionTypes = [
  stateTransitionTypes.IDENTITY_CREATE,
  stateTransitionTypes.IDENTITY_TOP_UP,
];
AbstractStateTransition.dataContractTransitionTypes = [
  stateTransitionTypes.DATA_CONTRACT_CREATE,
];

module.exports = AbstractStateTransition;
