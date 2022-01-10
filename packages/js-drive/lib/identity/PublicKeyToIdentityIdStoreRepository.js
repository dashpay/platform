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
   * @param {GroveDBTransaction} [transaction]
   *
   * @return {Promise<PublicKeyToIdentityIdStoreRepository>}
   */
  async store(publicKeyHash, identityId, transaction = undefined) {
    const identityIdsSerialized = await this.fetchBuffer(publicKeyHash, transaction);

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
        { transaction },
      );
    }
    return this;
  }

  /**
   * Fetch serialized identity ids by public key hash from database
   *
   * @param {Buffer} publicKeyHash
   * @param {GroveDBTransaction} [transaction]
   *
   * @return {Promise<Buffer|null>}
   */
  async fetchBuffer(publicKeyHash, transaction = undefined) {
    return this.storage.get(
      PublicKeyToIdentityIdStoreRepository.TREE_PATH,
      publicKeyHash,
      { transaction },
    );
  }

  /**
   * Fetch deserialized identity ids by public key hash from database
   *
   * @param {Buffer} publicKeyHash
   * @param {GroveDBTransaction} [transaction]
   *
   * @return {Promise<Identifier[]>}
   */
  async fetch(publicKeyHash, transaction = undefined) {
    const identityIdsSerialized = this.fetchBuffer(publicKeyHash, transaction);

    if (!identityIdsSerialized) {
      return [];
    }

    const identityIds = cbor.decode(identityIdsSerialized);

    return identityIds.map((id) => new Identifier(id));
  }

  /**
   * @return {Promise<PublicKeyToIdentityIdStoreRepository>}
   */
  async createTree() {
    await this.storage.createTree([], PublicKeyToIdentityIdStoreRepository.TREE_PATH[0]);

    return this;
  }
}

PublicKeyToIdentityIdStoreRepository.TREE_PATH = [Buffer.from('publicKeysToIdentityIds')];

module.exports = PublicKeyToIdentityIdStoreRepository;
