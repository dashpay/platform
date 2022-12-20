const Identity = require('@dashevo/dpp/lib/identity/Identity');

const PreCalculatedOperation = require('@dashevo/dpp/lib/stateTransition/fee/operations/PreCalculatedOperation');
const StorageResult = require('../storage/StorageResult');

class IdentityStoreRepository {
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
   * Create Identity in database
   *
   * @param {Identity} identity
   * @param {RawBlockInfo} blockInfo
   * @param {Object} [options]
   * @param {boolean} [options.useTransaction=false]
   * @param {boolean} [options.dryRun=false]
   *
   * @return {Promise<StorageResult<void>>}
   */
  async create(identity, blockInfo, options = {}) {
    try {
      const feeResult = await this.storage.getDrive().insertIdentity(
        identity,
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
          identity_id: identity.id.toString(),
          useTransaction: Boolean(options.useTransaction),
          appHash: (await this.storage.getRootHash(options)).toString('hex'),
        }, 'createContract');
      }
    }
  }

  /**
   * Remove balance from identity in database
   *
   * @param {Identifier} identityId
   * @param {number} amount
   * @param {RawBlockInfo} blockInfo
   * @param {Object} [options]
   * @param {boolean} [options.useTransaction=false]
   * @param {boolean} [options.dryRun=false]
   *
   * @return {Promise<StorageResult<void>>}
   */
  async updateAddToIdentityBalance(identityId, amount, blockInfo, options = {}) {
    try {
      const feeResult = await this.storage.getDrive().addToIdentityBalance(
        identityId,
        amount,
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
        }, 'createContract');
      }
    }
  }

  /**
   * Remove balance from identity in database
   *
   * @param {Identifier} identityId
   * @param {number} requiredAmount
   * @param {number} desiredAmount
   * @param {RawBlockInfo} blockInfo
   * @param {Object} [options]
   * @param {boolean} [options.useTransaction=false]
   * @param {boolean} [options.dryRun=false]
   *
   * @return {Promise<StorageResult<void>>}
   */
  async updateRemoveFromIdentityBalance(
    identityId,
    requiredAmount,
    desiredAmount,
    blockInfo,
    options = {},
  ) {
    try {
      const feeResult = await this.storage.getDrive().removeFromIdentityBalance(
        identityId,
        requiredAmount,
        desiredAmount,
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
        }, 'createContract');
      }
    }
  }

  /**
   * Remove balance from identity in database
   *
   * @param {Identifier} identityId
   * @param {number} amount
   * @param {RawBlockInfo} blockInfo
   * @param {Object} [options]
   * @param {boolean} [options.useTransaction=false]
   * @param {boolean} [options.dryRun=false]
   *
   * @return {Promise<StorageResult<void>>}
   */
  async updateAddBalance(identityId, amount, blockInfo, options = {}) {
    try {
      const feeResult = await this.storage.getDrive().addToIdentityBalance(
          identityId,
          amount,
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
        }, 'createContract');
      }
    }
  }

  /**
   * Add keys to an already existing Identity
   *
   * @param {Identifier} identityId
   * @param {} keys
   * @param {RawBlockInfo} blockInfo
   * @param {Object} [options]
   * @param {boolean} [options.useTransaction=false]
   * @param {boolean} [options.dryRun=false]
   *
   * @return {Promise<StorageResult<void>>}
   */
  async addKeysToIdentity(
      identityId,
      keys,
      blockInfo,
      options = {},
  ) {
    try {
      const feeResult = await this.storage.getDrive().removeFromIdentityBalance(
          identityId,
          requiredAmount,
          desiredAmount,
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
        }, 'createContract');
      }
    }
  }

  /**
   * Fetch identity by id from database
   *
   * @param {Identifier} id
   * @param {Object} [options]
   * @param {boolean} [options.useTransaction=false]
   * @param {boolean} [options.dryRun=false]
   * @return {Promise<StorageResult<null|Identity>>}
   */
  async fetch(id, options = { }) {
    if (options.dryRun) {
      return new StorageResult(null, []);
    }

    let encodedIdentityResult;
    try {
      encodedIdentityResult = await this.storage.get(
        IdentityStoreRepository.TREE_PATH.concat([id.toBuffer()]),
        IdentityStoreRepository.IDENTITY_KEY,
        options,
      );
    } catch (e) {
      if (!e.message.startsWith('grovedb: path parent layer not found')) {
        throw e;
      }

      encodedIdentityResult = new StorageResult(
        null,
        [],
      );
    }

    if (encodedIdentityResult.isNull()) {
      return encodedIdentityResult;
    }

    const [protocolVersion, rawIdentity] = this.decodeProtocolEntity(
      encodedIdentityResult.getValue(),
    );

    rawIdentity.protocolVersion = protocolVersion;

    return new StorageResult(
      new Identity(rawIdentity),
      encodedIdentityResult.getOperations(),
    );
  }

  /**
   * Prove identity by id
   *
   * @param {Identifier} id
   * @param {Object} [options]
   * @param {boolean} [options.useTransaction=false]
   *
   * @return {Promise<StorageResult<Buffer|null>>}
   * */
  async prove(id, options) {
    return this.proveMany([id], options);
  }

  /**
   * Prove identity by ids
   *
   * @param {Identifier[]} ids
   * @param {Object} [options]
   * @param {boolean} [options.useTransaction=false]
   *
   * @return {Promise<StorageResult<Buffer>>}
   * */
  async proveMany(ids, options) {
    const items = ids.map((id) => ({
      type: 'key',
      key: id.toBuffer(),
    }));

    return this.storage.proveQuery({
      path: IdentityStoreRepository.TREE_PATH,
      query: {
        query: {
          items,
          subqueryKey: IdentityStoreRepository.IDENTITY_KEY,
        },
      },
    }, options);
  }
}

IdentityStoreRepository.TREE_PATH = [Buffer.from([0])];

IdentityStoreRepository.IDENTITY_KEY = Buffer.from([0]);

module.exports = IdentityStoreRepository;
