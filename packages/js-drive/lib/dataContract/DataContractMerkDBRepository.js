const MerkDbTransaction = require('../merkDb/MerkDbTransaction');

class DataContractMerkDBRepository {
  /**
   *
   * @param {Merk} dataContractMerkDB
   * @param {DashPlatformProtocol} noStateDpp
   */
  constructor(dataContractMerkDB, noStateDpp) {
    this.db = dataContractMerkDB;
    this.dpp = noStateDpp;
  }

  /**
   * Store Data Contract into database
   *
   * @param {DataContract} dataContract
   * @param {MerkDbTransaction} [transaction]
   * @return {Promise<DataContractMerkDBRepository>}
   */
  async store(dataContract, transaction = undefined) {
    let db;

    if (transaction) {
      db = transaction.db;

      db.put(
        dataContract.getId(),
        dataContract.toBuffer(),
      );
    } else {
      db = this.db;

      db
        .batch()
        .put(
          dataContract.getId(),
          dataContract.toBuffer(),
        )
        .commitSync();
    }

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
    const db = transaction ? transaction.db : this.db;

    try {
      const encodedDataContract = db.getSync(id);

      return this.dpp.dataContract.createFromBuffer(
        encodedDataContract,
        { skipValidation: true },
      );
    } catch (e) {
      if (e.message.startsWith('key not found')) {
        return null;
      }

      throw e;
    }
  }

  /**
   * Creates new transaction instance
   *
   * @return {MerkDbTransaction}
   */
  createTransaction() {
    return new MerkDbTransaction(this.db);
  }
}

module.exports = DataContractMerkDBRepository;
