const { PublicKey } = require('@dashevo/dashcore-lib');

class IdentityPublicKey {
  /**
   * @param {RawIdentityPublicKey} [rawIdentityPublicKey]
   */
  constructor(rawIdentityPublicKey = undefined) {
    this.enabled = true;

    if (rawIdentityPublicKey) {
      this.setId(rawIdentityPublicKey.id)
        .setType(rawIdentityPublicKey.type)
        .setData(rawIdentityPublicKey.data)
        .setEnabled(rawIdentityPublicKey.isEnabled);
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
   * Set base64 encoded public key
   *
   * @param {string} data
   * @return {IdentityPublicKey}
   */
  setData(data) {
    this.data = data;

    return this;
  }

  /**
   * Get base64 encoded public key
   *
   * @return {string}
   */
  getData() {
    return this.data;
  }

  /**
   * Disable/enable public key
   *
   * @param {boolean} enabled
   * @return {IdentityPublicKey}
   */
  setEnabled(enabled) {
    this.enabled = enabled;

    return this;
  }

  /**
   * Is Public key enabled?
   *
   * @return {boolean}
   */
  isEnabled() {
    return this.enabled;
  }

  /**
   * Get original public key hash
   *
   * @returns {string}
   */
  hash() {
    return new PublicKey(this.getData()).hash
      .toString('hex');
  }

  /**
   * Get JSON representation
   *
   * @return {RawIdentityPublicKey}
   */
  toJSON() {
    return {
      id: this.getId(),
      type: this.getType(),
      data: this.getData(),
      isEnabled: this.isEnabled(),
    };
  }
}

IdentityPublicKey.TYPES = {
  ECDSA_SECP256K1: 0,
};

module.exports = IdentityPublicKey;
