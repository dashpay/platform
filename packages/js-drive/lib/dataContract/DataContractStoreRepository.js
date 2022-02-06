const DataContract = require('@dashevo/dpp/lib/dataContract/DataContract');

class DataContractStoreRepository {
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
   * Store Data Contract into database
   *
   * @param {DataContract} dataContract
   * @param {boolean} [useTransaction=false]
   * @return {Promise<number>}
   */
  async store(dataContract, useTransaction = false) {
    return this.storage.getDrive().applyContract(dataContract, useTransaction);
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
      return null;
    }

    const [, rawDataContract] = this.decodeProtocolEntity(encodedDataContract);

    return new DataContract(rawDataContract);
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

    return this;
  }
}

DataContractStoreRepository.TREE_PATH = [Buffer.from([1])];
DataContractStoreRepository.DATA_CONTRACT_KEY = Buffer.from([0]);
DataContractStoreRepository.DOCUMENTS_TREE_KEY = Buffer.from([0]);

module.exports = DataContractStoreRepository;
