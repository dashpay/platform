const hash = require('../util/hash');
const { encode } = require('../util/serializer');
const IdentityPublicKey = require('./IdentityPublicKey');

class Identity {
  /**
   * @param {RawIdentity} [rawIdentity]
   */
  constructor(rawIdentity = undefined) {
    this.publicKeys = [];

    if (rawIdentity) {
      this.id = rawIdentity.id;
      this.type = rawIdentity.type;

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
   * @return {number}
   */
  getType() {
    return this.type;
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
    return this.publicKeys.find((publicKey) => publicKey.getId() === keyId);
  }

  /**
   * @return {RawIdentity}
   */
  toJSON() {
    return {
      id: this.getId(),
      type: this.getType(),
      publicKeys: this.getPublicKeys()
        .map((publicKey) => publicKey.toJSON()),
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
}

Identity.TYPES = {
  USER: 1,
  APPLICATION: 2,
};

Identity.MAX_RESERVED_TYPE = 32767;

Identity.TYPES_ENUM = Object.values(Identity.TYPES);

module.exports = Identity;
