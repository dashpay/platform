const hash = require('../util/hash');
const { encode } = require('../util/serializer');
const IdentityPublicKey = require('./IdentityPublicKey');
const Identifier = require('../identifier/Identifier');

class Identity {
  /**
   * @param {RawIdentity} rawIdentity
   */
  constructor(rawIdentity) {
    if (Object.prototype.hasOwnProperty.call(rawIdentity, 'protocolVersion')) {
      this.protocolVersion = rawIdentity.protocolVersion;
    }

    if (Object.prototype.hasOwnProperty.call(rawIdentity, 'id')) {
      this.id = Identifier.from(rawIdentity.id);
    }

    if (Object.prototype.hasOwnProperty.call(rawIdentity, 'publicKeys')) {
      this.publicKeys = rawIdentity.publicKeys.map((rawPublicKey) => (
        new IdentityPublicKey(rawPublicKey)
      ));
    }

    if (Object.prototype.hasOwnProperty.call(rawIdentity, 'balance')) {
      this.balance = rawIdentity.balance;
    }

    if (Object.prototype.hasOwnProperty.call(rawIdentity, 'revision')) {
      this.revision = rawIdentity.revision;
    }
  }

  /**
   * @returns {number}
   */
  getProtocolVersion() {
    return this.protocolVersion;
  }

  /**
   * @return {Identifier}
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
   * Get plain object representation
   *
   * @return {RawIdentity}
   */
  toObject() {
    return {
      protocolVersion: this.getProtocolVersion(),
      id: this.getId().toBuffer(),
      publicKeys: this.getPublicKeys()
        .map((publicKey) => publicKey.toObject()),
      balance: this.getBalance(),
      revision: this.getRevision(),
    };
  }

  /**
   * Get JSON representation
   *
   * @return {JsonIdentity}
   */
  toJSON() {
    return {
      protocolVersion: this.getProtocolVersion(),
      id: this.getId().toString(),
      publicKeys: this.getPublicKeys()
        .map((publicKey) => publicKey.toJSON()),
      balance: this.getBalance(),
      revision: this.getRevision(),
    };
  }

  /**
   * @return {Buffer}
   */
  toBuffer() {
    const serializedData = this.toObject();
    delete serializedData.protocolVersion;

    const protocolVersionUInt32 = Buffer.alloc(4);
    protocolVersionUInt32.writeUInt32BE(this.getProtocolVersion(), 0);

    return Buffer.concat([protocolVersionUInt32, encode(serializedData)]);
  }

  /**
   * @return {Buffer}
   */
  hash() {
    return hash(this.toBuffer());
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
   * Set asset lock proof
   *
   * @param {InstantAssetLockProof|ChainAssetLockProof} assetLockProof
   * @return {Identity}
   */
  setAssetLockProof(assetLockProof) {
    this.assetLockProof = assetLockProof;

    return this;
  }

  /**
   * Get asset lock proof
   *
   * @return {InstantAssetLockProof|ChainAssetLockProof}
   */
  getAssetLockProof() {
    return this.assetLockProof;
  }

  /**
   * Get Identity revision
   *
   * @return {number}
   */
  getRevision() {
    return this.revision;
  }

  /**
   * Set Identity revision
   *
   * @param {number} revision
   * @return {Identity}
   */
  setRevision(revision) {
    this.revision = revision;

    return this;
  }

  /**
   * Set metadata
   * @param {Metadata} metadata
   */
  setMetadata(metadata) {
    this.metadata = metadata;
  }

  /**
   * Get metadata
   * @returns {Metadata|null}
   */
  getMetadata() {
    return this.metadata;
  }
}

/**
 * @typedef {Object} RawIdentity
 * @property {number} protocolVersion
 * @property {Buffer} id
 * @property {RawIdentityPublicKey[]} publicKeys
 * @property {number} balance
 * @property {number} revision
 */

/**
 * @typedef {Object} JsonIdentity
 * @property {number} protocolVersion
 * @property {string} id
 * @property {JsonIdentityPublicKey[]} publicKeys
 * @property {number} balance
 * @property {number} revision
 */

module.exports = Identity;
