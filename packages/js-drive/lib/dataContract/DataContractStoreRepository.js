class DataContractStoreRepository {
  /**
   *
   * @param {MerkDbStore} dataContractsStore
   * @param {DashPlatformProtocol} noStateDpp
   */
  constructor(dataContractsStore, noStateDpp) {
    this.storage = dataContractsStore;
    this.dpp = noStateDpp;
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

    return this.dpp.dataContract.createFromBuffer(
      encodedDataContract,
      { skipValidation: true },
    );
  }
}

module.exports = DataContractStoreRepository;
