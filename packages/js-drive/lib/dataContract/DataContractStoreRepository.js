class DataContractStoreRepository {
  /**
   *
   * @param {MerkDbStore} dataContractsStore
   * @param {AwilixContainer} container
   */
  constructor(dataContractsStore, container) {
    this.storage = dataContractsStore;
    this.container = container;
  }

  /**
   * Store Data Contract into database
   *
   * @param {DataContract} dataContract
   * @param {MerkDbTransaction} [transaction]
   * @return {Promise<DataContractStoreRepository>}
   */
  async store(dataContract, transaction = undefined) {
    this.storage.put(
      dataContract.getId(),
      dataContract.toBuffer(),
      transaction,
    );

    return this;
  }

  /**
   * Fetch Data Contract by ID from database
   *
   * @param {Identifier} id
   * @param {MerkDbTransaction} [transaction]
   * @return {Promise<null|DataContract>}
   */
  async fetch(id, transaction = undefined) {
    const encodedDataContract = this.storage.get(id, transaction);

    if (!encodedDataContract) {
      return null;
    }

    const dpp = this.container.resolve(transaction ? 'transactionalDpp' : 'dpp');

    return dpp.dataContract.createFromBuffer(
      encodedDataContract,
      { skipValidation: true },
    );
  }
}

module.exports = DataContractStoreRepository;
