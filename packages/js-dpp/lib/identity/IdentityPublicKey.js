const { crypto: { Hash } } = require('@dashevo/dashcore-lib');

const EmptyPublicKeyDataError = require('./errors/EmptyPublicKeyDataError');
const InvalidIdentityPublicKeyTypeError = require('../stateTransition/errors/InvalidIdentityPublicKeyTypeError');

class IdentityPublicKey {
  /**
   * @param {RawIdentityPublicKey} [rawIdentityPublicKey]
   */
  constructor(rawIdentityPublicKey = { }) {
    if (Object.prototype.hasOwnProperty.call(rawIdentityPublicKey, 'id')) {
      this.setId(rawIdentityPublicKey.id);
    }

    if (Object.prototype.hasOwnProperty.call(rawIdentityPublicKey, 'type')) {
      this.setType(rawIdentityPublicKey.type);
    }

    if (Object.prototype.hasOwnProperty.call(rawIdentityPublicKey, 'purpose')) {
      this.setPurpose(rawIdentityPublicKey.purpose);
    }

    if (Object.prototype.hasOwnProperty.call(rawIdentityPublicKey, 'securityLevel')) {
      this.setSecurityLevel(rawIdentityPublicKey.securityLevel);
    }

    if (Object.prototype.hasOwnProperty.call(rawIdentityPublicKey, 'data')) {
      this.setData(rawIdentityPublicKey.data);
    }

    if (Object.prototype.hasOwnProperty.call(rawIdentityPublicKey, 'readOnly')) {
      this.setReadOnly(rawIdentityPublicKey.readOnly);
    }

    if (Object.prototype.hasOwnProperty.call(rawIdentityPublicKey, 'disabledAt')) {
      this.setDisabledAt(rawIdentityPublicKey.disabledAt);
    }

    if (Object.prototype.hasOwnProperty.call(rawIdentityPublicKey, 'signature')) {
      this.setSignature(rawIdentityPublicKey.signature);
    }
  }

  /**
   * Get key ID
   *
   * @return {number}
   */
  getId() {
    return this.id;
  }

  /**
   * Set key ID
   *
   * @param {number} id
   * @return {IdentityPublicKey}
   */
  setId(id) {
    this.id = id;

    return this;
  }

  /**
   * Get key type
   *
   * @return {number}
   */
  getType() {
    return this.type;
  }

  /**
   * Set key type
   *
   * @param {number} type
   * @return {IdentityPublicKey}
   */
  setType(type) {
    this.type = type;

    return this;
  }

  /**
   * Set raw public key
   *
   * @param {Buffer} data
   * @return {IdentityPublicKey}
   */
  setData(data) {
    this.data = data;

    return this;
  }

  /**
   * Get raw public key
   *
   * @return {Buffer}
   */
  getData() {
    return this.data;
  }

  /**
   * Set the raw purpose value. A uint8 number
   *
   * @param {number} purpose
   * @return {IdentityPublicKey}
   */
  setPurpose(purpose) {
    this.purpose = purpose;

    return this;
  }

  /**
   * Get the raw purpose value. A uint8 number
   *
   * @return number
   */
  getPurpose() {
    return this.purpose;
  }

  /**
   * Set the raw security level. A uint8 number
   *
   * @param {number} securityLevel
   * @return {IdentityPublicKey}
   */
  setSecurityLevel(securityLevel) {
    this.securityLevel = securityLevel;

    return this;
  }

  /**
   * Get the raw security level value. A uint8 number
   *
   * @return number
   */
  getSecurityLevel() {
    return this.securityLevel;
  }

  /**
   * Set readOnly flag
   *
   * @param {boolean} readOnly
   * @return {IdentityPublicKey}
   */
  setReadOnly(readOnly) {
    this.readOnly = readOnly;

    return this;
  }

  /**
   * Get readOnly flag
   *
   * @return {boolean}
   */
  isReadOnly() {
    return this.readOnly;
  }

  /**
   * Set disabledAt timestamp
   *
   * @param {number} disabledAt
   * @return {IdentityPublicKey}
   */
  setDisabledAt(disabledAt) {
    this.disabledAt = disabledAt;

    return this;
  }

  /**
   * Get disabledAt timestamp
   *
   * @return {number}
   */
  getDisabledAt() {
    return this.disabledAt;
  }

  /**
   * Is public key disabled
   */
  isDisabled() {
    return this.getDisabledAt() !== undefined;
  }

  /**
   * Set signature
   *
   * @param {Buffer} signature
   * @returns {IdentityPublicKey}
   */
  setSignature(signature) {
    this.signature = signature;

    return this;
  }

  /**
   * Get signature
   *
   * @returns {Buffer}
   */
  getSignature() {
    return this.signature;
  }

  /**
   * Get the original public key hash
   *
   * @return {Buffer}
   */
  hash() {
    if (!this.getData()) {
      throw new EmptyPublicKeyDataError();
    }

    switch (this.getType()) {
      case IdentityPublicKey.TYPES.BLS12_381:
      case IdentityPublicKey.TYPES.ECDSA_SECP256K1: {
        return Hash.sha256ripemd160(this.getData());
      }
      case IdentityPublicKey.TYPES.ECDSA_HASH160:
      case IdentityPublicKey.TYPES.BIP13_SCRIPT_HASH:
        return this.getData();
      default:
        throw new InvalidIdentityPublicKeyTypeError(this.getType());
    }
  }

  /**
   * Get a plain object representation
   *
   * @param {Object} [options]
   * @param {Object} [options.skipSignature=false]
   *
   * @return {RawIdentityPublicKey}
   */
  toObject(options = {}) {
    const result = {
      id: this.getId(),
      type: this.getType(),
      purpose: this.getPurpose(),
      securityLevel: this.getSecurityLevel(),
      data: this.getData(),
      readOnly: this.isReadOnly(),
    };

    if (this.getDisabledAt() !== undefined) {
      result.disabledAt = this.getDisabledAt();
    }

    if (!options.skipSignature && this.signature !== undefined) {
      result.signature = this.signature;
    }

    return result;
  }

  /**
   * Get a JSON representation
   *
   * @return {JsonIdentityPublicKey}
   */
  toJSON() {
    const result = {
      ...this.toObject(),
      data: this.getData().toString('base64'),
    };

    if (this.signature) {
      result.signature = this.signature.toString('base64');
    }

    return result;
  }

  /**
   * Check if public ket security level is MASTER
   *
   * @returns {boolean}
   */
  isMaster() {
    return this.getSecurityLevel() === IdentityPublicKey.SECURITY_LEVELS.MASTER;
  }
}

/**
 * @typedef {Object} RawIdentityPublicKey
 * @property {number} id
 * @property {number} type
 * @property {number} purpose
 * @property {number} securityLevel
 * @property {Buffer} data
 * @property {boolean} readOnly
 * @property {number} [disabledAt]
 * @property {Buffer} [signature]
 */

/**
 * @typedef {Object} JsonIdentityPublicKey
 * @property {number} id
 * @property {number} purpose
 * @property {number} securityLevel
 * @property {number} type
 * @property {string} data
 * @property {boolean} readOnly
 * @property {number} [disabledAt]
 * @property {string} [signature]
 */

IdentityPublicKey.TYPES = {
  ECDSA_SECP256K1: 0,
  BLS12_381: 1,
  ECDSA_HASH160: 2,
  BIP13_SCRIPT_HASH: 3,
};

IdentityPublicKey.PURPOSES = {
  AUTHENTICATION: 0,
  ENCRYPTION: 1,
  DECRYPTION: 2,
  WITHDRAW: 3,
};

IdentityPublicKey.SECURITY_LEVELS = {
  MASTER: 0,
  CRITICAL: 1,
  HIGH: 2,
  MEDIUM: 3,
};

IdentityPublicKey.ALLOWED_SECURITY_LEVELS = {};
IdentityPublicKey.ALLOWED_SECURITY_LEVELS[IdentityPublicKey.PURPOSES.AUTHENTICATION] = [
  IdentityPublicKey.SECURITY_LEVELS.MASTER,
  IdentityPublicKey.SECURITY_LEVELS.CRITICAL,
  IdentityPublicKey.SECURITY_LEVELS.HIGH,
  IdentityPublicKey.SECURITY_LEVELS.MEDIUM,
];
IdentityPublicKey.ALLOWED_SECURITY_LEVELS[IdentityPublicKey.PURPOSES.ENCRYPTION] = [
  IdentityPublicKey.SECURITY_LEVELS.MEDIUM,
];
IdentityPublicKey.ALLOWED_SECURITY_LEVELS[IdentityPublicKey.PURPOSES.DECRYPTION] = [
  IdentityPublicKey.SECURITY_LEVELS.MEDIUM,
];
IdentityPublicKey.ALLOWED_SECURITY_LEVELS[IdentityPublicKey.PURPOSES.WITHDRAW] = [
  IdentityPublicKey.SECURITY_LEVELS.CRITICAL,
];

module.exports = IdentityPublicKey;
