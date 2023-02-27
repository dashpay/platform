const {
  PublicKey,
  PrivateKey,
  Signer: { sign, verifySignature, verifyHashSignature },
} = require('@dashevo/dashcore-lib');

const varint = require('varint');
const StateTransitionIsNotSignedError = require(
  './errors/StateTransitionIsNotSignedError',
);

const stateTransitionTypes = require('./stateTransitionTypes');

const hashModule = require('../util/hash');
const serializer = require('../util/serializer');

const IdentityPublicKey = require('../identity/IdentityPublicKey');
const InvalidIdentityPublicKeyTypeError = require('./errors/InvalidIdentityPublicKeyTypeError');
const blsPrivateKeyFactory = require('../bls/blsPrivateKeyFactory');
const blsPublicKeyFactory = require('../bls/blsPublicKeyFactory');
const BlsSignatures = require('../bls/bls');
const StateTransitionExecutionContext = require('./StateTransitionExecutionContext');

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

    this.executionContext = new StateTransitionExecutionContext();
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
   * @abstract
   * @return {Identifier}
   */
  getOwnerId() {
    throw new Error('Not implemented');
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

    const protocolVersionBytes = Buffer.from(varint.encode(this.getProtocolVersion()));

    return Buffer.concat([protocolVersionBytes, serializer.encode(serializedData)]);
  }

  /**
   * Returns hex string with Data Contract hash
   *
   * @param {Object} [options]
   * @param {boolean} [options.skipSignature=false]
   * @return {Buffer}
   */
  hash(options = {}) {
    const { hash } = hashModule;

    return hash(this.toBuffer(options));
  }

  /**
   * Sign data with private key
   * @param {string|Buffer|Uint8Array|PrivateKey} privateKey string must be hex or base58
   * @param {number} keyType private key type
   * @return {Promise<AbstractStateTransition>}
   */
  async signByPrivateKey(privateKey, keyType) {
    const data = this.toBuffer({ skipSignature: true });

    switch (keyType) {
      case IdentityPublicKey.TYPES.ECDSA_SECP256K1:
      case IdentityPublicKey.TYPES.ECDSA_HASH160: {
        const privateKeyModel = new PrivateKey(privateKey);

        this.setSignature(sign(data, privateKeyModel));

        break;
      }
      case IdentityPublicKey.TYPES.BLS12_381: {
        const privateKeyModel = await blsPrivateKeyFactory(privateKey);
        const { BasicSchemeMPL } = await BlsSignatures.getInstance();

        const blsSignature = BasicSchemeMPL.sign(privateKeyModel, new Uint8Array(data));

        const blsSignatureBuffer = Buffer.from(blsSignature.serialize());

        privateKeyModel.delete();
        blsSignature.delete();

        this.setSignature(blsSignatureBuffer);
        break;
      }
      default:
        throw new InvalidIdentityPublicKeyTypeError(keyType);
    }

    return this;
  }

  /**
   * Verify signature by public key
   *
   * @param {Buffer} publicKey
   * @param publicKeyType
   *
   * @returns {Promise<boolean>}
   */
  async verifyByPublicKey(publicKey, publicKeyType) {
    switch (publicKeyType) {
      case IdentityPublicKey.TYPES.ECDSA_SECP256K1:
        return this.verifyECDSASignatureByPublicKey(publicKey);
      case IdentityPublicKey.TYPES.ECDSA_HASH160:
        return this.verifyESDSAHash160SignatureByPublicKeyHash(publicKey);
      case IdentityPublicKey.TYPES.BLS12_381:
        return this.verifyBLSSignatureByPublicKey(publicKey);
      default:
        throw new InvalidIdentityPublicKeyTypeError(publicKeyType);
    }
  }

  /**
   * @protected
   * @param {Buffer} publicKeyHash
   * @return {boolean}
   */
  verifyESDSAHash160SignatureByPublicKeyHash(publicKeyHash) {
    const signature = this.getSignature();
    if (!signature) {
      throw new StateTransitionIsNotSignedError(this);
    }

    const hash = this.hash({ skipSignature: true });

    let isSignatureVerified;
    try {
      isSignatureVerified = verifyHashSignature(hash, signature, publicKeyHash);
    } catch (e) {
      isSignatureVerified = false;
    }

    return isSignatureVerified;
  }

  /**
   * Verify signature with public key
   * @protected
   * @param {string|Buffer|Uint8Array|PublicKey} publicKey string must be hex or base58
   * @returns {boolean}
   */
  verifyECDSASignatureByPublicKey(publicKey) {
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
   * Verify signature with public key
   * @protected
   * @param {string|Buffer|Uint8Array|PublicKey} publicKey string must be hex
   * @returns {Promise<boolean>}
   */
  async verifyBLSSignatureByPublicKey(publicKey) {
    const signature = this.getSignature();
    if (!signature) {
      throw new StateTransitionIsNotSignedError(this);
    }

    const data = this.toBuffer({ skipSignature: true });

    const publicKeyModel = await blsPublicKeyFactory(publicKey);

    const { G2Element, BasicSchemeMPL } = await BlsSignatures.getInstance();

    let blsSignature;
    let result;

    try {
      blsSignature = G2Element.fromBytes(Uint8Array.from(signature));

      result = BasicSchemeMPL.verify(publicKeyModel, new Uint8Array(data), blsSignature);
      // eslint-disable-next-line no-useless-catch
    } catch (e) {
      throw e;
    } finally {
      if (blsSignature) {
        blsSignature.delete();
      }
    }

    return result;
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

  /**
   * Set state transition execution context
   *
   * @param {StateTransitionExecutionContext} executionContext
   */
  setExecutionContext(executionContext) {
    this.executionContext = executionContext;
  }

  /**
   * Get state transition execution context
   *
   * @return {StateTransitionExecutionContext}
   */
  getExecutionContext() {
    return this.executionContext;
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
  stateTransitionTypes.IDENTITY_UPDATE,
];
AbstractStateTransition.dataContractTransitionTypes = [
  stateTransitionTypes.DATA_CONTRACT_CREATE,
  stateTransitionTypes.DATA_CONTRACT_UPDATE,
];

module.exports = AbstractStateTransition;
