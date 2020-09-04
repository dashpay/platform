const hash = require('../util/hash');
const { encode } = require('../util/serializer');
const IdentityPublicKey = require('./IdentityPublicKey');

class Identity {
  /**
   * @param {RawIdentity} rawIdentity
   */
  constructor(rawIdentity) {
    if (Object.prototype.hasOwnProperty.call(rawIdentity, 'protocolVersion')) {
      this.protocolVersion = rawIdentity.protocolVersion;
    }

    if (Object.prototype.hasOwnProperty.call(rawIdentity, 'id')) {
      this.id = rawIdentity.id;
    }

    if (Object.prototype.hasOwnProperty.call(rawIdentity, 'publicKeys')) {
      this.publicKeys = rawIdentity.publicKeys.map((rawPublicKey) => (
        new IdentityPublicKey(rawPublicKey)
      ));
    }

    if (Object.prototype.hasOwnProperty.call(rawIdentity, 'balance')) {
      this.balance = rawIdentity.balance;
    }
  }

  /**
   * @returns {number}
   */
  getProtocolVersion() {
    return this.protocolVersion;
  }

  /**
   * @return {string}
   */
  getId() {
    return this.id;
  }

  /**
   * @param {IdentityPublicKey[]} publicKeys
   * @return {Identity}
   */
  setPublicKeys(publicKeys) {
    this.publicKeys = publicKeys;

    return this;
  }

  /**
   * @return {IdentityPublicKey[]}
   */
  getPublicKeys() {
    return this.publicKeys;
  }

  /**
   * Returns a public key for a given id
   *
   * @param {number} keyId
   * @return {IdentityPublicKey}
   */
  getPublicKeyById(keyId) {
    return this.publicKeys.find((k) => k.getId() === keyId);
  }

  /**
   * @return {RawIdentity}
   */
  toJSON() {
    return {
      protocolVersion: this.getProtocolVersion(),
      id: this.getId(),
      publicKeys: this.getPublicKeys()
        .map((publicKey) => publicKey.toJSON()),
      balance: this.getBalance(),
    };
  }

  /**
   * @return {Buffer}
   */
  serialize() {
    return encode(this.toJSON());
  }

  /**
   * @return {string}
   */
  hash() {
    return hash(this.serialize())
      .toString('hex');
  }

  /**
   * Returns balance
   * @returns {number}
   */
  getBalance() {
    return this.balance;
  }

  /**
   * Set Identity balance
   *
   * @param {number} balance
   * @return {Identity}
   */
  setBalance(balance) {
    this.balance = balance;

    return this;
  }

  /**
   * Increase balance
   *
   * @param {number} amount
   * @return {number}
   */
  increaseBalance(amount) {
    this.balance += amount;

    return this.balance;
  }

  /**
   * Reduce balance
   *
   * @param {number} amount
   * @return {number}
   */
  reduceBalance(amount) {
    this.balance -= amount;

    return this.balance;
  }

  /**
   * Set locked out point
   *
   * @param {Buffer} lockedOutPoint
   * @return {Identity}
   */
  setLockedOutPoint(lockedOutPoint) {
    this.lockedOutPoint = lockedOutPoint;

    return this;
  }

  /**
   * Get locked out point
   *
   * @return {Buffer}
   */
  getLockedOutPoint() {
    return this.lockedOutPoint;
  }
}

/**
 * @typedef {Object} RawIdentity
 * @property {number} protocolVersion
 * @property {string} id
 * @property {RawIdentityPublicKey[]} publicKeys
 * @property {number} balance
 */

Identity.PROTOCOL_VERSION = 0;

module.exports = Identity;
