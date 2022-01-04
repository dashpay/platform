const cbor = require('cbor');

const Identifier = require('@dashevo/dpp/lib/Identifier');

class PublicKeyToIdentityIdStoreRepository {
  /**
   *
   * @param {MerkDbStore} publicKeyToIdentityIdStore
   */
  constructor(publicKeyToIdentityIdStore) {
    this.storage = publicKeyToIdentityIdStore;
  }

  /**
   * Store public key to identity ids map into database
   *
   * @param {Buffer} publicKeyHash
   * @param {Identifier} identityId
   * @param {MerkDbTransaction} [transaction]
   *
   * @return {Promise<PublicKeyToIdentityIdStoreRepository>}
   */
  async store(publicKeyHash, identityId, transaction = undefined) {
    const identityIdsSerialized = this.storage.get(publicKeyHash, transaction);

    let identityIds = [];
    if (identityIdsSerialized) {
      identityIds = cbor.decode(identityIdsSerialized);
    }

    if (!identityIds.includes(identityId)) {
      identityIds.push(identityId.toBuffer());

      this.storage.put(
        publicKeyHash,
        cbor.encode(identityIds),
        transaction,
      );
    }

    return this;
  }

  /**
   * Fetch serialized identity ids by public key hash from database
   *
   * @param {Buffer} publicKeyHash
   * @param {MerkDbTransaction} [transaction]
   *
   * @return {Promise<Buffer|null>}
   */
  async fetch(publicKeyHash, transaction = undefined) {
    return this.storage.get(publicKeyHash, transaction);
  }

  /**
   * Fetch deserialized identity ids by public key hash from database
   *
   * @param {Buffer} publicKeyHash
   * @param {MerkDbTransaction} [transaction]
   *
   * @return {Promise<Identifier[]>}
   */
  async fetchDeserialized(publicKeyHash, transaction = undefined) {
    const identityIdsSerialized = this.storage.get(publicKeyHash, transaction);

    if (!identityIdsSerialized) {
      return [];
    }

    const identityIds = cbor.decode(identityIdsSerialized);

    return identityIds.map((id) => new Identifier(id));
  }
}

module.exports = PublicKeyToIdentityIdStoreRepository;
