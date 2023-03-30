const StorageResult = require('../storage/StorageResult');

class IdentityPublicKeyStoreRepository {
  /**
   *
   * @param {GroveDBStore} groveDBStore
   * @param {decodeProtocolEntity} decodeProtocolEntity
   * @param {WebAssembly.Instance} dppWasm
   */
  constructor(groveDBStore, decodeProtocolEntity, dppWasm) {
    this.storage = groveDBStore;
    this.decodeProtocolEntity = decodeProtocolEntity;
    this.dppWasm = dppWasm;
  }

  /**
   * Add keys to an already existing Identity
   *
   * @param {Identifier} identityId
   * @param {IdentityPublicKey[]} keys
   * @param {BlockInfo} blockInfo
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
        blockInfo.toObject(),
        Boolean(options.useTransaction),
        Boolean(options.dryRun),
      );

      return new StorageResult(
        undefined,
        [
          new this.dppWasm.PreCalculatedOperation(
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
   * Disable keys in already existing Identity
   *
   * @param {Identifier} identityId
   * @param {number[]} keyIds
   * @param {number} disabledAt
   * @param {BlockInfo} blockInfo
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
        blockInfo.toObject(),
        Boolean(options.useTransaction),
        Boolean(options.dryRun),
      );

      return new StorageResult(
        undefined,
        [
          new this.dppWasm.PreCalculatedOperation(
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
        }, 'disable');
      }
    }
  }
}

IdentityPublicKeyStoreRepository.TREE_PATH = [Buffer.from([2])];

module.exports = IdentityPublicKeyStoreRepository;
