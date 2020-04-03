const LevelDbTransaction = require('../levelDb/LevelDBTransaction');

class DataContractLevelDBRepository {
  /**
   *
   * @param {LevelUP} dataContractLevelDB
   * @param {DashPlatformProtocol} noStateDpp
   */
  constructor(dataContractLevelDB, noStateDpp) {
    this.db = dataContractLevelDB;
    this.dpp = noStateDpp;
  }

  /**
   * Store Data Contract into database
   *
   * @param {DataContract} dataContract
   * @param {LevelDBTransaction} [transaction]
   * @return {Promise<DataContractLevelDBRepository>}
   */
  async store(dataContract, transaction = undefined) {
    const db = transaction ? transaction.db : this.db;

    await db.put(
      this.addKeyPrefix(dataContract.getId()),
      dataContract.serialize(),
      { asBuffer: true },
    );

    return this;
  }

  /**
   * Fetch Data Contract by ID from database
   *
   * @param {string} id
   * @param {LevelDBTransaction} [transaction]
   * @return {Promise<null|DataContract>}
   */
  async fetch(id, transaction = undefined) {
    const db = transaction ? transaction.db : this.db;

    try {
      const encodedDataContract = await db.get(
        this.addKeyPrefix(id),
      );

      return this.dpp.dataContract.createFromSerialized(
        encodedDataContract,
        { skipValidation: true },
      );
    } catch (e) {
      if (e.type === 'NotFoundError') {
        return null;
      }

      throw e;
    }
  }

  /**
   * Creates new transaction instance
   *
   * @return {LevelDBTransaction}
   */
  createTransaction() {
    return new LevelDbTransaction(this.db);
  }

  /**
   * Get DB key by identity id
   *
   * @private
   * @param {string} id
   * @return {string}
   */
  addKeyPrefix(id) {
    return `${DataContractLevelDBRepository.KEY_PREFIX}:${id}`;
  }
}

DataContractLevelDBRepository.KEY_PREFIX = 'dataContract';

module.exports = DataContractLevelDBRepository;
