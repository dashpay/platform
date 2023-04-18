const { PreCalculatedOperation } = require('@dashevo/wasm-dpp');

const StorageResult = require('../storage/StorageResult');

class IdentityBalanceStoreRepository {
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
   * Fetch balance by id from database
   *
   * @param {Identifier} id
   * @param {Object} [options]
   * @param {boolean} [options.useTransaction=false]
   * @param {BlockInfo} [options.blockInfo]
   * @param {boolean} [options.dryRun=false]
   *
   * @return {Promise<StorageResult<number|null>>}
   */
  async fetch(id, options = { }) {
    if (options && options.blockInfo) {
      const [balance, feeResult] = await this.storage.getDrive().fetchIdentityBalanceWithCosts(
        id,
        options.blockInfo.toObject(),
        Boolean(options.useTransaction),
      );

      return new StorageResult(
        balance,
        [
          new PreCalculatedOperation(
            feeResult.storageFee,
            feeResult.processingFee,
            feeResult.feeRefunds,
          ),
        ],
      );
    }

    const balance = await this.storage.getDrive().fetchIdentityBalance(
      id,
      Boolean(options.useTransaction),
    );

    return new StorageResult(
      balance,
      [],
    );
  }

  /**
   * Fetch identity by id from database
   *
   * @param {Identifier} id
   * @param {BlockInfo} blockInfo
   * @param {Object} [options]
   * @param {boolean} [options.useTransaction=false]
   * @param {boolean} [options.dryRun=false]
   *
   * @return {Promise<StorageResult<number|null>>}
   */
  async fetchWithDebt(id, blockInfo, options = {}) {
    const [
      balance,
      feeResult,
    ] = await this.storage.getDrive().fetchIdentityBalanceIncludeDebtWithCosts(
      id,
      blockInfo.toObject(),
      Boolean(options.useTransaction),
      Boolean(options.dryRun),
    );

    return new StorageResult(
      balance,
      [
        new PreCalculatedOperation(
          feeResult.storageFee,
          feeResult.processingFee,
          feeResult.feeRefunds,
        ),
      ],
    );
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
  async add(identityId, amount, blockInfo, options = {}) {
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
          new PreCalculatedOperation(
            feeResult.storageFee,
            feeResult.processingFee,
            feeResult.feeRefunds,
          ),
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
   * Apply fees to identity balance in database
   *
   * @param {Identifier} identityId
   * @param {FeeResult} fees
   * @param {Object} [options]
   * @param {boolean} [options.useTransaction=false]
   *
   * @return {Promise<StorageResult<FeeResult>>}
   */
  async applyFees(
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
        }, 'applyFees');
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
  async remove(
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
          new PreCalculatedOperation(
            feeResult.storageFee,
            feeResult.processingFee,
            feeResult.feeRefunds,
          ),
        ],
      );
    } finally {
      if (this.logger) {
        this.logger.trace({
          identity_id: identityId.toString(),
          useTransaction: Boolean(options.useTransaction),
          appHash: (await this.storage.getRootHash(options)).toString('hex'),
        }, 'remove');
      }
    }
  }
}

module.exports = IdentityBalanceStoreRepository;
