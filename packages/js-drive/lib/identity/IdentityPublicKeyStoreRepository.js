const PreCalculatedOperation = require('@dashevo/dpp/lib/stateTransition/fee/operations/PreCalculatedOperation');
const Identity = require('@dashevo/dpp/lib/identity/Identity');
const StorageResult = require('../storage/StorageResult');

class IdentityPublicKeyStoreRepository {
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
    if (options.dryRun) {
      return new StorageResult([], []);
    }

    const result = await this.storage.query({
      path: IdentityPublicKeyStoreRepository.TREE_PATH.concat([publicKeyHash]),
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
   * Add keys to an already existing Identity
   *
   * @param {Identifier} identityId
   * @param {IdentityPublicKey[]} keys
   * @param {RawBlockInfo} blockInfo
   * @param {Object} [options]
   * @param {boolean} [options.useTransaction=false]
   * @param {boolean} [options.dryRun=false]
   *
   * @return {Promise<StorageResult<void>>}
   */
  async add(
    identityId,
    keys,
    blockInfo,
    options = {},
  ) {
    try {
      const feeResult = await this.storage.getDrive().addKeysToIdentity(
        identityId,
        keys,
        blockInfo,
        Boolean(options.useTransaction),
        Boolean(options.dryRun),
      );

      return new StorageResult(
        undefined,
        [
          new PreCalculatedOperation(feeResult),
        ],
      );
    } finally {
      if (this.logger) {
        this.logger.trace({
          identity_id: identityId.toString(),
          useTransaction: Boolean(options.useTransaction),
          appHash: (await this.storage.getRootHash(options)).toString('hex'),
        }, 'add');
      }
    }
  }

  /**
   * Disable keys in already existing Identity
   *
   * @param {Identifier} identityId
   * @param {number[]} keyIds
   * @param {number} disabledAt
   * @param {RawBlockInfo} blockInfo
   * @param {Object} [options]
   * @param {boolean} [options.useTransaction=false]
   * @param {boolean} [options.dryRun=false]
   *
   * @return {Promise<StorageResult<void>>}
   */
  async disable(
    identityId,
    keyIds,
    disabledAt,
    blockInfo,
    options = {},
  ) {
    try {
      const feeResult = await this.storage.getDrive().disableIdentityKeys(
        identityId,
        keyIds,
        disabledAt,
        blockInfo,
        Boolean(options.useTransaction),
        Boolean(options.dryRun),
      );

      return new StorageResult(
        undefined,
        [
          new PreCalculatedOperation(feeResult),
        ],
      );
    } finally {
      if (this.logger) {
        this.logger.trace({
          identity_id: identityId.toString(),
          useTransaction: Boolean(options.useTransaction),
          appHash: (await this.storage.getRootHash(options)).toString('hex'),
        }, 'disable');
      }
    }
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
    if (options.dryRun) {
      return new StorageResult([], []);
    }

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
    if (options.dryRun) {
      return new StorageResult([], []);
    }

    const items = publicKeyHashes.map((publicKeyHash) => ({
      type: 'key',
      key: publicKeyHash,
    }));

    return this.storage.query({
      path: IdentityPublicKeyStoreRepository.TREE_PATH,
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
 * Prove identities by multiple public key hashes
 *
 * @param {Buffer[]} publicKeyHashes
 * @param {Object} [options]
 * @param {boolean} [options.useTransaction=false]
 *
 * @return {Promise<StorageResult<Buffer>>}
 */
  async proveMany(publicKeyHashes, options = {}) {
    const items = publicKeyHashes.map((publicKeyHash) => ({
      type: 'key',
      key: publicKeyHash,
    }));

    return this.storage.proveQuery({
      path: IdentityPublicKeyStoreRepository.TREE_PATH,
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

IdentityPublicKeyStoreRepository.TREE_PATH = [Buffer.from([2])];

module.exports = IdentityPublicKeyStoreRepository;
