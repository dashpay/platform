const cbor = require('cbor');

const Identifier = require('@dashevo/dpp/lib/Identifier');

class PublicKeyToIdentityIdStoreRepository {
  /**
   *
   * @param {GroveDBStore} groveDBStore
   */
  constructor(groveDBStore) {
    this.storage = groveDBStore;
  }

  /**
   * Store public key to identity ids map into database
   *
   * @param {Buffer} publicKeyHash
   * @param {Identifier} identityId
   * @param {boolean} [useTransaction=false]
   *
   * @return {Promise<PublicKeyToIdentityIdStoreRepository>}
   */
  async store(publicKeyHash, identityId, useTransaction = false) {
    const identityIdsSerialized = await this.fetchBuffer(publicKeyHash, useTransaction);

    let identityIds = [];
    if (identityIdsSerialized) {
      identityIds = cbor.decode(identityIdsSerialized);
    }

    if (identityIds.find((id) => id.equals(identityId)) === undefined) {
      identityIds.push(identityId.toBuffer());

      await this.storage.put(
        PublicKeyToIdentityIdStoreRepository.TREE_PATH,
        publicKeyHash,
        cbor.encode(identityIds),
        { useTransaction },
      );
    }
    return this;
  }

  /**
   * Fetch serialized identity ids by public key hash from database
   *
   * @param {Buffer} publicKeyHash
   * @param {boolean} [useTransaction=false]
   *
   * @return {Promise<Buffer|null>}
   */
  async fetchBuffer(publicKeyHash, useTransaction = false) {
    return this.storage.get(
      PublicKeyToIdentityIdStoreRepository.TREE_PATH,
      publicKeyHash,
      { useTransaction },
    );
  }

  /**
   * Fetch deserialized identity ids by public key hash from database
   *
   * @param {Buffer} publicKeyHash
   * @param {boolean} [useTransaction=false]
   *
   * @return {Promise<Identifier[]>}
   */
  async fetch(publicKeyHash, useTransaction = false) {
    const identityIdsSerialized = await this.fetchBuffer(publicKeyHash, useTransaction);

    if (!identityIdsSerialized) {
      return [];
    }

    const identityIds = cbor.decode(identityIdsSerialized);

    return identityIds.map((id) => new Identifier(id));
  }

  /**
   * @param {Object} [options]
   * @param {boolean} [options.useTransaction=false]
   * @param {boolean} [options.skipIfExists]
   *
   * @return {Promise<PublicKeyToIdentityIdStoreRepository>}
   */
  async createTree(options = {}) {
    await this.storage.createTree([], PublicKeyToIdentityIdStoreRepository.TREE_PATH[0], options);

    return this;
  }
}

PublicKeyToIdentityIdStoreRepository.TREE_PATH = [Buffer.from([2])];

module.exports = PublicKeyToIdentityIdStoreRepository;
