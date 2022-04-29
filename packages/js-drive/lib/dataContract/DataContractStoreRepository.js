const DataContract = require('@dashevo/dpp/lib/dataContract/DataContract');
const { createHash } = require('crypto');

const Write = require('../fees/Write');
const Read = require('../fees/Read');
const PreCalculatedOperation = require('../fees/PreCalculatedOperation');

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
   * @return {Promise<void>}
   */
  async store(dataContract, useTransaction = false) {
    try {
      const [cpuCost, storageCost] = await this.storage.getDrive().applyContract(
        dataContract,
        new Date('2022-03-17T15:08:26.132Z'),
        useTransaction,
      );
      return {
        result: this,
        operations: [
          new PreCalculatedOperation(
            cpuCost,
            storageCost,
          ),
        ],
      };
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
   * @return {Promise<null|DataContract>}
   */
  async fetch(id, useTransaction = false) {
    const encodedDataContract = await this.storage.get(
      DataContractStoreRepository.TREE_PATH.concat([id.toBuffer()]),
      DataContractStoreRepository.DATA_CONTRACT_KEY,
      { useTransaction },
    );

    if (!encodedDataContract) {
      return {
        result: null,
        operations: [
          new Read(
            DataContractStoreRepository.DATA_CONTRACT_KEY.length,
            DataContractStoreRepository.TREE_PATH.concat([id.toBuffer()]).reduce((size, pathItem) => size += pathItem.length, 0).length,
            0,
          ),
        ],
      };
    }

    const [protocolVersion, rawDataContract] = this.decodeProtocolEntity(encodedDataContract);

    rawDataContract.protocolVersion = protocolVersion;

    return {
      result: new DataContract(rawDataContract),
      operations: [
        new Read(
          DataContractStoreRepository.DATA_CONTRACT_KEY.length,
          DataContractStoreRepository.TREE_PATH.concat([id.toBuffer()]).reduce((size, pathItem) => size += pathItem.length, 0).length,
          encodedDataContract.length,
        ),
      ],
    };
  }

  /**
   * @param {Object} [options]
   * @param {boolean} [options.useTransaction=false]
   * @param {boolean} [options.skipIfExists]
   *
   * @return {Promise<DataContractStoreRepository>}
   */
  async createTree(options = {}) {
    await this.storage.createTree([], DataContractStoreRepository.TREE_PATH[0], options);

    return {
      result: this,
      operations: [
        new Write(
          DataContractStoreRepository.TREE_PATH[0].length,
          32,
        ),
      ],
    };
  }
}

DataContractStoreRepository.TREE_PATH = [Buffer.from([1])];
DataContractStoreRepository.DATA_CONTRACT_KEY = Buffer.from([0]);
DataContractStoreRepository.DOCUMENTS_TREE_KEY = Buffer.from([0]);

module.exports = DataContractStoreRepository;
