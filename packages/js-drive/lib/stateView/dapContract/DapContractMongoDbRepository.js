const DapContract = require('./DapContract');

class DapContractMongoDbRepository {
  /**
   * @param {Db} mongoDb
   * @param {sanitizeData} sanitizeData
   */
  constructor(mongoDb, { sanitize, unsanitize }) {
    this.collection = mongoDb.collection('dapContracts');
    this.sanitize = sanitize;
    this.unsanitize = unsanitize;
  }

  /**
   * Find DapContract by dapId
   *
   * @param {string} dapId
   * @returns {Promise<DapContract|null>}
   */
  async find(dapId) {
    const result = await this.collection.findOne({ _id: dapId });

    if (!result) {
      return null;
    }

    const {
      dapId: id,
      dapName,
      packetHash,
      schema,
    } = this.unsanitize(result);

    return new DapContract(id, dapName, packetHash, schema);
  }

  /**
   * Store DapContract entity
   *
   * @param {DapContract} dapContract
   * @returns {Promise}
   */
  async store(dapContract) {
    const dapContractData = dapContract.toJSON();

    return this.collection.updateOne(
      { _id: dapContractData.dapId },
      { $set: this.sanitize(dapContractData) },
      { upsert: true },
    );
  }
}

module.exports = DapContractMongoDbRepository;
