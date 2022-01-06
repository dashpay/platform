const decodeProtocolEntityFactory = require('@dashevo/dpp/lib/decodeProtocolEntityFactory');
const DataContract = require('@dashevo/dpp/lib/dataContract/DataContract');

const decodeProtocolEntity = decodeProtocolEntityFactory();

class DataContractStoreRepository {
  /**
   *
   * @param {GroveDBStore} groveDBStore
   */
  constructor(groveDBStore) {
    this.storage = groveDBStore;
  }

  /**
   * Store Data Contract into database
   *
   * @param {DataContract} dataContract
   * @param {GroveDBTransaction} [transaction]
   * @return {Promise<DataContractStoreRepository>}
   */
  async store(dataContract, transaction = undefined) {
    this.storage.put(
      dataContract.getId().toBuffer(),
      dataContract.toBuffer(),
      transaction,
    );

    return this;
  }

  /**
   * Fetch Data Contract by ID from database
   *
   * @param {Identifier} id
   * @param {GroveDBTransaction} [transaction]
   * @return {Promise<null|DataContract>}
   */
  async fetch(id, transaction = undefined) {
    const encodedDataContract = this.storage.get(id.toBuffer(), transaction);

    if (!encodedDataContract) {
      return null;
    }

    const [, rawDataContract] = decodeProtocolEntity(encodedDataContract);

    return new DataContract(rawDataContract);
  }
}

module.exports = DataContractStoreRepository;
