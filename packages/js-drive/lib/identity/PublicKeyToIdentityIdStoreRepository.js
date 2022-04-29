const cbor = require('cbor');

const Identifier = require('@dashevo/dpp/lib/Identifier');

const Write = require('@dashevo/dpp/lib/stateTransition/fees/operations/WriteOperation');
const Read = require('@dashevo/dpp/lib/stateTransition/fees/operations/ReadOperation');

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
    const { result: identityIdsSerialized, operations: ops } = await this.fetchBuffer(publicKeyHash, useTransaction);

    let identityIds = [];
    if (identityIdsSerialized) {
      identityIds = cbor.decode(identityIdsSerialized);
    }

    const operations = [
      ...ops,
    ];

    if (identityIds.find((id) => id.equals(identityId)) === undefined) {
      identityIds.push(identityId.toBuffer());

      const data = cbor.encode(identityIds);

      await this.storage.put(
        PublicKeyToIdentityIdStoreRepository.TREE_PATH,
        publicKeyHash,
        data,
        { useTransaction },
      );

      operations.push(
        new Write(
          PublicKeyToIdentityIdStoreRepository.TREE_PATH.reduce((size, pathItem) => size += pathItem.length, 0) + publicKeyHash.length,
          data.length,
        ),
      );
    }

    return {
      result: this,
      operations,
    };
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
    const result = await this.storage.get(
      PublicKeyToIdentityIdStoreRepository.TREE_PATH,
      publicKeyHash,
      { useTransaction },
    );

    const operations = [
      new Read(
        publicKeyHash.length,
        PublicKeyToIdentityIdStoreRepository.TREE_PATH.reduce((size, pathItem) => size += pathItem.length, 0),
        result.reduce((size, id) => size += id.length, 0),
      ),
    ];

    return {
      result,
      operations,
    };
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
    const { result: identityIdsSerialized, operations } = await this.fetchBuffer(publicKeyHash, useTransaction);

    if (!identityIdsSerialized) {
      return {
        result: [],
        operations,
      };
    }

    const identityIds = cbor.decode(identityIdsSerialized);

    return {
      result: identityIds.map((id) => new Identifier(id)),
      operations,
    };
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

    return {
      result: this,
      operations: [
        new Write(PublicKeyToIdentityIdStoreRepository.TREE_PATH[0].length, 32),
      ]
    };
  }
}

PublicKeyToIdentityIdStoreRepository.TREE_PATH = [Buffer.from([2])];

module.exports = PublicKeyToIdentityIdStoreRepository;
