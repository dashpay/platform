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

    if (Object.prototype.hasOwnProperty.call(rawIdentityPublicKey, 'data')) {
      this.setData(rawIdentityPublicKey.data);
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
   * Get original public key hash
   *
   * @returns {Buffer}
   */
  hash() {
    if (!this.getData()) {
      throw new EmptyPublicKeyDataError();
    }

    const originalPublicKey = new PublicKey(
      this.getData(),
    );

    return originalPublicKey.hash;
  }

  /**
   * Get plain object representation
   *
   * @return {RawIdentityPublicKey}
   */
  toObject() {
    return {
      id: this.getId(),
      type: this.getType(),
      data: this.getData(),
    };
  }

  /**
   * Get JSON representation
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
 * @property {Buffer} data
 */

/**
 * @typedef {Object} JsonIdentityPublicKey
 * @property {number} id
 * @property {number} type
 * @property {string} data
 */

IdentityPublicKey.TYPES = {
  ECDSA_SECP256K1: 0,
  BLS12_381: 1,
};

module.exports = IdentityPublicKey;
