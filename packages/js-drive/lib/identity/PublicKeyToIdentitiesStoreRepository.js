const Identity = require('@dashevo/dpp/lib/identity/Identity');
const StorageResult = require('../storage/StorageResult');
const IdentityStoreRepository = require('./IdentityStoreRepository');

class PublicKeyToIdentitiesStoreRepository {
  /**
   *
   * @param {GroveDBStore} groveDBStore
   * @param {decodeProtocolEntity} decodeProtocolEntity
   */
  constructor(groveDBStore, decodeProtocolEntity) {
    this.storage = groveDBStore;
    this.decodeProtocolEntity = decodeProtocolEntity;
  }

  /**
   * Store public key to identity ids map into database
   *
   * @param {Buffer} publicKeyHash
   * @param {Identifier} identityId
   * @param {Object} [options]
   * @param {boolean} [options.useTransaction=false]
   * @param {boolean} [options.dryRun=false]
   *
   * @return {Promise<StorageResult<void>>}
   */
  async store(publicKeyHash, identityId, options = {}) {
    const treeResult = await this.storage.createTree(
      PublicKeyToIdentitiesStoreRepository.TREE_PATH,
      publicKeyHash,
      {
        ...options,
        skipIfExists: true,
      },
    );

    const key = identityId.toBuffer();

    const referenceResult = await this.storage.putReference(
      PublicKeyToIdentitiesStoreRepository.TREE_PATH.concat([publicKeyHash]),
      key,
      IdentityStoreRepository.TREE_PATH.concat([key, IdentityStoreRepository.IDENTITY_KEY]),
      options,
    );

    return new StorageResult(
      undefined,
      treeResult.getOperations().concat(referenceResult.getOperations()),
    );
  }

  /**
   * Fetch deserialized identities by public key hash
   *
   * @param {Buffer} publicKeyHash
   * @param {Object} [options]
   * @param {boolean} [options.useTransaction=false]
   * @param {boolean} [options.dryRun=false]
   *
   * @return {Promise<StorageResult<Identity[]>>}
   */
  async fetch(publicKeyHash, options = {}) {
    const result = await this.storage.query({
      path: PublicKeyToIdentitiesStoreRepository.TREE_PATH.concat([publicKeyHash]),
      query: {
        query: {
          items: [
            {
              type: 'rangeFull',
            },
          ],
        },
      },
    }, options);

    return new StorageResult(
      result.getValue().map((serializedIdentity) => {
        const [protocolVersion, rawIdentity] = this.decodeProtocolEntity(
          serializedIdentity,
        );

        rawIdentity.protocolVersion = protocolVersion;

        return new Identity(rawIdentity);
      }),
      result.getOperations(),
    );
  }

  /**
   * Fetch deserialized identities by multiple public key hashes
   *
   * @param {Buffer[]} publicKeyHashes
   * @param {Object} [options]
   * @param {boolean} [options.useTransaction=false]
   * @param {boolean} [options.dryRun=false]
   *
   * @return {Promise<StorageResult<Identity[]>>}
   */
  async fetchMany(publicKeyHashes, options = {}) {
    const result = await this.fetchManyBuffers(publicKeyHashes, options);

    return new StorageResult(
      result.getValue().map((serializedIdentity) => {
        const [protocolVersion, rawIdentity] = this.decodeProtocolEntity(
          serializedIdentity,
        );

        rawIdentity.protocolVersion = protocolVersion;

        return new Identity(rawIdentity);
      }),
      result.getOperations(),
    );
  }

  /**
   * Fetch serialized identities by multiple public key hashes
   *
   * @param {Buffer[]} publicKeyHashes
   * @param {Object} [options]
   * @param {boolean} [options.useTransaction=false]
   * @param {boolean} [options.dryRun=false]
   *
   * @return {Promise<StorageResult<Buffer[]>>}
   */
  async fetchManyBuffers(publicKeyHashes, options = {}) {
    const items = publicKeyHashes.map((publicKeyHash) => ({
      type: 'key',
      key: publicKeyHash,
    }));

    return this.storage.query({
      path: PublicKeyToIdentitiesStoreRepository.TREE_PATH,
      query: {
        query: {
          items,
          subquery: {
            items: [
              {
                type: 'rangeFull',
              },
            ],
          },
        },
      },
    }, options);
  }

  /**
   * @param {Object} [options]
   * @param {boolean} [options.useTransaction=false]
   * @param {boolean} [options.skipIfExists=false]
   * @param {boolean} [options.dryRun=false]
   *
   * @return {Promise<StorageResult<void>>}
   */
  async createTree(options = {}) {
    return this.storage.createTree(
      [],
      PublicKeyToIdentitiesStoreRepository.TREE_PATH[0],
      options,
    );
  }

  /**
 * Prove identities by multiple public key hashes
 *
 * @param {Buffer[]} publicKeyHashes
 * @param {Object} [options]
 * @param {boolean} [options.useTransaction=false]
 *
 * @return {Promise<StorageResult<Buffer[]>>}
 */
  async proveMany(publicKeyHashes, options = {}) {
    const items = publicKeyHashes.map((publicKeyHash) => ({
      type: 'key',
      key: publicKeyHash,
    }));

    return this.storage.prove({
      path: PublicKeyToIdentitiesStoreRepository.TREE_PATH,
      query: {
        query: {
          items,
          subquery: {
            items: [
              {
                type: 'rangeFull',
              },
            ],
          },
        },
      },
    }, options);
  }
}

PublicKeyToIdentitiesStoreRepository.TREE_PATH = [Buffer.from([2])];

module.exports = PublicKeyToIdentitiesStoreRepository;
