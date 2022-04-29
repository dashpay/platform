const DataContract = require('@dashevo/dpp/lib/dataContract/DataContract');
const { createHash } = require('crypto');

const PreCalculatedOperation = require('@dashevo/dpp/lib/stateTransition/fees/operations/PreCalculatedOperation');
const RepositoryResult = require('../storage/RepositoryResult');

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
   * @param {boolean} [useTransaction=false]
   * @return {Promise<RepositoryResult<void>>}
   */
  async store(dataContract, useTransaction = false) {
    try {
      const [storageCost, cpuCost] = await this.storage.getDrive().applyContract(
        dataContract,
        new Date('2022-03-17T15:08:26.132Z'),
        useTransaction,
      );

      return new RepositoryResult(
        undefined,
        [
          new PreCalculatedOperation(
            storageCost,
            cpuCost,
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
          useTransaction: Boolean(useTransaction),
          appHash: (await this.storage.getRootHash({ useTransaction })).toString('hex'),
        }, 'applyContract');
      }
    }
  }

  /**
   * Fetch Data Contract by ID from database
   *
   * @param {Identifier} id
   * @param {boolean} [useTransaction=false]
   * @return {Promise<RepositoryResult<null|DataContract>>}
   */
  async fetch(id, useTransaction = false) {
    const encodedDataContractResult = await this.storage.get(
      DataContractStoreRepository.TREE_PATH.concat([id.toBuffer()]),
      DataContractStoreRepository.DATA_CONTRACT_KEY,
      { useTransaction },
    );

    if (!encodedDataContractResult.getResult()) {
      return new RepositoryResult(
        null,
        encodedDataContractResult.getOperations(),
      );
    }

    const [protocolVersion, rawDataContract] = this.decodeProtocolEntity(
      encodedDataContractResult.getResult(),
    );

    rawDataContract.protocolVersion = protocolVersion;

    return new RepositoryResult(
      new DataContract(rawDataContract),
      encodedDataContractResult.getOperations(),
    );
  }

  /**
   * @param {Object} [options]
   * @param {boolean} [options.useTransaction=false]
   * @param {boolean} [options.skipIfExists]
   *
   * @return {Promise<RepositoryResult<void>>}
   */
  async createTree(options = {}) {
    const createTreeResult = await this.storage.createTree(
      [],
      DataContractStoreRepository.TREE_PATH[0],
      options,
    );

    return new RepositoryResult(
      undefined,
      createTreeResult.getOperations(),
    );
  }
}

DataContractStoreRepository.TREE_PATH = [Buffer.from([1])];
DataContractStoreRepository.DATA_CONTRACT_KEY = Buffer.from([0]);
DataContractStoreRepository.DOCUMENTS_TREE_KEY = Buffer.from([0]);

module.exports = DataContractStoreRepository;
