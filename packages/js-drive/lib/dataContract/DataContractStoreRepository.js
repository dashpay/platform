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
   * @return {Promise<DataContractStoreRepository>}
   */
  async store(dataContract, useTransaction = false) {
    /**
     * Store contract
     */

    // Create contract tree
    await this.storage.createTree(
      DataContractStoreRepository.TREE_PATH,
      dataContract.getId().toBuffer(),
      { useTransaction },
    );

    // Store contract under Data Contract key
    const contractTreePath = DataContractStoreRepository.TREE_PATH
      .concat([dataContract.getId().toBuffer()]);

    await this.storage.put(
      contractTreePath,
      DataContractStoreRepository.DATA_CONTRACT_KEY,
      dataContract.toBuffer(),
      { useTransaction },
    );

    /**
     * Create document type trees
     */
    const promises = Object.entries(dataContract.getDocuments())
      .map(async ([documentType, documentDefinition]) => {
        // Create document type tree
        await this.storage.createTree(
          contractTreePath,
          Buffer.from(documentType),
          { useTransaction, skipIfExists: true },
        );

        const documentTypeTreePath = contractTreePath.concat([Buffer.from(documentType)]);

        // Create IDs tree
        await this.storage.createTree(
          documentTypeTreePath,
          DataContractStoreRepository.DOCUMENTS_TREE_KEY,
          { useTransaction, skipIfExists: true },
        );

        // Create first indexed property trees
        const firstIndexedProperties = (documentDefinition.indices || []).map((indexDefinition) => (
          Object.keys(indexDefinition.properties[0])[0]
        ));

        const uniqueFirstIndexedProperties = [...new Set(firstIndexedProperties)];

        await Promise.all(uniqueFirstIndexedProperties.map(async (indexedProperty) => {
          // Create tree for indexed property
          await this.storage.createTree(
            documentTypeTreePath,
            Buffer.from(indexedProperty),
            { useTransaction, skipIfExists: true },
          );

          // Create tree for ID references
          await this.storage.createTree(
            documentTypeTreePath.concat([Buffer.from(indexedProperty)]),
            DataContractStoreRepository.DOCUMENTS_TREE_KEY,
            { useTransaction, skipIfExists: true },
          );
        }));
      });

    await Promise.all(promises);

    return this;
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
   * @return {Promise<DataContractStoreRepository>}
   */
  async createTree() {
    await this.storage.createTree([], DataContractStoreRepository.TREE_PATH[0]);

    return this;
  }
}

DataContractStoreRepository.TREE_PATH = [Buffer.from('contracts')];
DataContractStoreRepository.DATA_CONTRACT_KEY = Buffer.from([0]);
DataContractStoreRepository.DOCUMENTS_TREE_KEY = Buffer.from([0]);

module.exports = DataContractStoreRepository;
