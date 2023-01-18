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
   * Fetch identity by id from database
   *
   * @param {Identifier} id
   * @param {Object} [options]
   * @param {boolean} [options.useTransaction=false]
   * @param {BlockInfo} [options.blockInfo]
   * @param {boolean} [options.dryRun=false]
   *
   * @return {Promise<StorageResult<null|Identity>>}
   */
  async fetch(id, options = { }) {
    if (options.dryRun) {
      return new StorageResult(
        null,
        [],
      );
    }

    if (options && options.blockInfo) {
      const [identity, feeResult] = await this.storage.getDrive().fetchIdentityWithCosts(
        id,
        options.blockInfo.epoch,
        Boolean(options.useTransaction),
      );

      return new StorageResult(
        identity,
        [new PreCalculatedOperation(feeResult)],
      );
    }

    const identity = await this.storage.getDrive().fetchIdentity(
      id,
      Boolean(options.useTransaction),
    );

    return new StorageResult(
      identity,
      [],
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
   * @return {Promise<StorageResult<Identity>>}
   */
  async fetchByPublicKeyHash(publicKeyHash, options = {}) {
    throw new Error('not implemented');
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
  async fetchManyByPublicKeyHashes(publicKeyHashes, options = {}) {
    throw new Error('not implemented');
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
  async proveManyByPublicKeyHashes(publicKeyHashes, options = {}) {
    throw new Error('not implemented');
  }

  /**
   * Create Identity in database
   *
   * @param {Identity} identity
   * @param {BlockInfo} blockInfo
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
        blockInfo.toObject(),
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
        }, 'create');
      }
    }
  }

  /**
   * Add to identity balance in database
   *
   * @param {Identifier} identityId
   * @param {number} amount
   * @param {BlockInfo} blockInfo
   * @param {Object} [options]
   * @param {boolean} [options.useTransaction=false]
   * @param {boolean} [options.dryRun=false]
   *
   * @return {Promise<StorageResult<void>>}
   */
  async addToBalance(identityId, amount, blockInfo, options = {}) {
    try {
      const feeResult = await this.storage.getDrive().addToIdentityBalance(
        identityId,
        amount,
        blockInfo.toObject(),
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
        }, 'addToBalance');
      }
    }
  }

  /**
   * Apply fees to identity balance in database
   *
   * @param {Identifier} identityId
   * @param {FeeResult} fees
   * @param {Object} [options]
   * @param {boolean} [options.useTransaction=false]
   *
   * @return {Promise<StorageResult<FeeResult>>}
   */
  async applyFeesToBalance(
    identityId,
    fees,
    options = {},
  ) {
    try {
      const feeResult = await this.storage.getDrive().applyFeesToIdentityBalance(
        identityId,
        fees,
        Boolean(options.useTransaction),
      );

      return new StorageResult(
        feeResult,
        [],
      );
    } finally {
      if (this.logger) {
        this.logger.trace({
          identity_id: identityId.toString(),
          useTransaction: Boolean(options.useTransaction),
          appHash: (await this.storage.getRootHash(options)).toString('hex'),
        }, 'applyFeesToBalance');
      }
    }
  }

  /**
   * Remove balance from identity in database
   *
   * @param {Identifier} identityId
   * @param {number} amount
   * @param {BlockInfo} blockInfo
   * @param {Object} [options]
   * @param {boolean} [options.useTransaction=false]
   * @param {boolean} [options.dryRun=false]
   *
   * @return {Promise<StorageResult<void>>}
   */
  async removeFromBalance(
    identityId,
    amount,
    blockInfo,
    options = {},
  ) {
    try {
      const feeResult = await this.storage.getDrive().removeFromIdentityBalance(
        identityId,
        amount,
        blockInfo.toObject(),
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
        }, 'removeFromBalance');
      }
    }
  }

  /**
   * Update identity revision in database
   *
   * @param {Identifier} identityId
   * @param {number} revision
   * @param {BlockInfo} blockInfo
   * @param {Object} [options]
   * @param {boolean} [options.useTransaction=false]
   * @param {boolean} [options.dryRun=false]
   *
   * @return {Promise<StorageResult<void>>}
   */
  async updateRevision(
    identityId,
    revision,
    blockInfo,
    options = {},
  ) {
    try {
      const feeResult = await this.storage.getDrive().updateIdentityRevision(
        identityId,
        revision,
        blockInfo.toObject(),
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
        }, 'updateRevision');
      }
    }
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
    throw new Error('No implemented');
  }
}

module.exports = IdentityStoreRepository;
