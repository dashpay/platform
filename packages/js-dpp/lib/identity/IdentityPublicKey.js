const { PublicKey } = require('@dashevo/dashcore-lib');

const EmptyPublicKeyDataError = require('./errors/EmptyPublicKeyDataError');

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
   * @return boolean
   */
  getReadOnly() {
    return this.readOnly;
  }

  /**
   * Get the original public key hash
   *
   * @returns {Buffer}
   */
  hash() {
    if (!this.getData()) {
      throw new EmptyPublicKeyDataError();
    }

    if (this.getType() === IdentityPublicKey.TYPES.ECDSA_HASH160) {
      return this.getData();
    }

    const originalPublicKey = new PublicKey(
      this.getData(),
    );

    return originalPublicKey.hash;
  }

  /**
   * Get a plain object representation
   *
   * @return {RawIdentityPublicKey}
   */
  toObject() {
    return {
      id: this.getId(),
      type: this.getType(),
      purpose: this.getPurpose(),
      securityLevel: this.getSecurityLevel(),
      data: this.getData(),
      readOnly: this.getReadOnly(),
    };
  }

  /**
   * Get a JSON representation
   *
   * @return {JsonIdentityPublicKey}
   */
  toJSON() {
    return {
      ...this.toObject(),
      data: this.getData().toString('base64'),
    };
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
 */

/**
 * @typedef {Object} JsonIdentityPublicKey
 * @property {number} id
 * @property {number} purpose
 * @property {number} securityLevel
 * @property {number} type
 * @property {string} data
 * @property {boolean} readOnly
 */

IdentityPublicKey.TYPES = {
  ECDSA_SECP256K1: 0,
  BLS12_381: 1,
  ECDSA_HASH160: 2,
};

IdentityPublicKey.PURPOSES = {
  AUTHENTICATION: 0,
  ENCRYPTION: 1,
  DECRYPTION: 2,
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

module.exports = IdentityPublicKey;
