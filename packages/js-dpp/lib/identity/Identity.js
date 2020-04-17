const hash = require('../util/hash');
const { encode } = require('../util/serializer');
const IdentityPublicKey = require('./IdentityPublicKey');

class Identity {
  /**
   * @param {RawIdentity} [rawIdentity]
   */
  constructor(rawIdentity = undefined) {
    this.publicKeys = [];
    this.balance = 0;

    if (rawIdentity) {
      this.id = rawIdentity.id;
      this.balance = rawIdentity.balance;

      if (rawIdentity.publicKeys) {
        this.setPublicKeys(
          rawIdentity.publicKeys.map((rawPublicKey) => new IdentityPublicKey(rawPublicKey)),
        );
      }
    }
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
   * @return {Identity}
   */
  increaseBalance(amount) {
    this.balance += amount;

    return this.balance;
  }

  /**
   * Reduce balance
   *
   * @param {number} amount
   * @return {Identity}
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

module.exports = Identity;
