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
   * Fetch identity by public key hash
   *
   * @param {Buffer} hash
   * @param {Object} [options]
   * @param {boolean} [options.useTransaction=false]
   *
   * @return {Promise<StorageResult<Identity|null>>}
   */
  async fetchByPublicKeyHash(hash, options = {}) {
    try {
      const [identity] = await this.storage.getDrive().fetchIdentitiesByPublicKeyHashes(
        [hash],
        Boolean(options.useTransaction),
      );

      return new StorageResult(
        identity,
        [],
      );
    } finally {
      if (this.logger) {
        this.logger.trace({
          hash,
          useTransaction: Boolean(options.useTransaction),
          appHash: (await this.storage.getRootHash(options)).toString('hex'),
        }, 'fetchManyByPublicKeyHashes');
      }
    }
  }

  /**
   * Fetch many identities by public key hashes
   *
   * @param {Buffer[]} hashes
   * @param {Object} [options]
   * @param {boolean} [options.useTransaction=false]
   *
   * @return {Promise<StorageResult<Array<Identity>>>}
   */
  async fetchManyByPublicKeyHashes(hashes, options = {}) {
    try {
      const identities = await this.storage.getDrive().fetchIdentitiesByPublicKeyHashes(
        hashes,
        Boolean(options.useTransaction),
      );

      return new StorageResult(
        identities,
        [],
      );
    } finally {
      if (this.logger) {
        this.logger.trace({
          hashes,
          useTransaction: Boolean(options.useTransaction),
          appHash: (await this.storage.getRootHash(options)).toString('hex'),
        }, 'fetchManyByPublicKeyHashes');
      }
    }
  }

  /**
   * Prove identities by multiple public key hashes
   *
   * @param {Buffer[]} hashes
   * @param {Object} [options]
   * @param {boolean} [options.useTransaction=false]
   *
   * @return {Promise<StorageResult<Buffer>>}
   */
  async proveManyByPublicKeyHashes(hashes, options = {}) {
    try {
      const proof = await this.storage.getDrive().proveIdentitiesByPublicKeyHashes(
        hashes,
        Boolean(options.useTransaction),
      );

      return new StorageResult(
        proof,
        [],
      );
    } finally {
      if (this.logger) {
        this.logger.trace({
          hashes,
          useTransaction: Boolean(options.useTransaction),
          appHash: (await this.storage.getRootHash(options)).toString('hex'),
        }, 'proveManyByPublicKeyHashes');
      }
    }
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
   * @return {Promise<StorageResult<Buffer>>}
   * */
  async prove(id, options = {}) {
    try {
      const proof = await this.storage.getDrive().proveIdentity(
        id,
        Boolean(options.useTransaction),
      );

      return new StorageResult(
        proof,
        [],
      );
    } finally {
      if (this.logger) {
        this.logger.trace({
          id,
          useTransaction: Boolean(options.useTransaction),
          appHash: (await this.storage.getRootHash(options)).toString('hex'),
        }, 'prove');
      }
    }
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
  async proveMany(ids, options = {}) {
    try {
      const proof = await this.storage.getDrive().proveManyIdentities(
        ids,
        Boolean(options.useTransaction),
      );

      return new StorageResult(
        proof,
        [],
      );
    } finally {
      if (this.logger) {
        this.logger.trace({
          ids,
          useTransaction: Boolean(options.useTransaction),
          appHash: (await this.storage.getRootHash(options)).toString('hex'),
        }, 'proveMany');
      }
    }
  }
}

module.exports = IdentityStoreRepository;
