const DataContract = require('@dashevo/dpp/lib/dataContract/DataContract');
const { createHash } = require('crypto');

const PreCalculatedOperation = require('@dashevo/dpp/lib/stateTransition/fee/operations/PreCalculatedOperation');
const StorageResult = require('../storage/StorageResult');

class DataContractStoreRepository {
  /**
   *
   * @param {GroveDBStore} groveDBStore
   * @param {decodeProtocolEntity} decodeProtocolEntity
   * @param {BaseLogger} [logger]
   */
  constructor(groveDBStore, decodeProtocolEntity, logger = undefined) {
    this.storage = groveDBStore;
    this.decodeProtocolEntity = decodeProtocolEntity;
    this.logger = logger;
  }

  /**
   * Store Data Contract into database
   *
   * @param {DataContract} dataContract
   * @param {BlockInfo} blockInfo
   * @param {Object} [options]
   * @param {boolean} [options.useTransaction=false]
   * @param {boolean} [options.dryRun=false]
   *
   * @return {Promise<StorageResult<void>>}
   */
  async store(dataContract, blockInfo, options = {}) {
    try {
      const { storageFee, processingFee } = await this.storage.getDrive().applyContract(
        dataContract,
        blockInfo,
        Boolean(options.useTransaction),
        Boolean(options.dryRun), // TODO rs-drive doesn't support this
      );

      return new StorageResult(
        undefined,
        [
          new PreCalculatedOperation(
            storageFee,
            processingFee,
          ),
        ],
      );
    } finally {
      if (this.logger) {
        this.logger.trace({
          dataContract: dataContract.toBuffer().toString('hex'),
          dataContractHash: createHash('sha256')
            .update(
              dataContract.toBuffer(),
            ).digest('hex'),
          useTransaction: Boolean(options.useTransaction),
          appHash: (await this.storage.getRootHash(options)).toString('hex'),
        }, 'applyContract');
      }
    }
  }

  /**
   * Fetch Data Contract by ID from database
   *
   * @param {Identifier} id
   * @param {Object} [options]
   * @param {boolean} [options.useTransaction=false]
   * @param {boolean} [options.dryRun=false]
   *
   * @return {Promise<StorageResult<null|DataContract>>}
   */
  async fetch(id, options = {}) {
    const result = await this.storage.get(
      DataContractStoreRepository.TREE_PATH.concat([id.toBuffer()]),
      DataContractStoreRepository.DATA_CONTRACT_KEY,
      {
        ...options,
        predictedValueSize: 16 * 1024, // Max size of State Transition
      },
    );

    if (result.isNull()) {
      return result;
    }

    const [protocolVersion, rawDataContract] = this.decodeProtocolEntity(
      result.getValue(),
    );

    rawDataContract.protocolVersion = protocolVersion;

    return new StorageResult(
      new DataContract(rawDataContract),
      result.getOperations(),
    );
  }

  /**
 * Prove Data Contract by ID from database
 *
 * @param {Identifier} id
 * @param {Object} [options]
 * @param {boolean} [options.useTransaction=false]
 * @return {Promise<StorageResult<Buffer|null>>}
 * */
  async prove(id, options) {
    return this.proveMany([id], options);
  }

  /**
 * Prove Data Contract by IDs from database
 *
 * @param {Identifier[]} ids
 * @param {Object} [options]
 * @param {boolean} [options.useTransaction=false]
 * @return {Promise<StorageResult<Buffer|null>>}
 * */
  async proveMany(ids, options) {
    const items = ids.map((id) => ({
      type: 'key',
      key: id.toBuffer(),
    }));

    return this.storage.proveQuery({
      path: DataContractStoreRepository.TREE_PATH,
      query: {
        query: {
          items,
          subqueryKey: DataContractStoreRepository.DATA_CONTRACT_KEY,
        },
      },
    }, options);
  }
}

DataContractStoreRepository.TREE_PATH = [Buffer.from([1])];
DataContractStoreRepository.DATA_CONTRACT_KEY = Buffer.from([0]);
DataContractStoreRepository.DOCUMENTS_TREE_KEY = Buffer.from([0]);

module.exports = DataContractStoreRepository;
