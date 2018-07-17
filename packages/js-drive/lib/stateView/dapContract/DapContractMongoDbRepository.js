const DapContract = require('./DapContract');

class DapContractMongoDbRepository {
  /**
   * @param {Db} mongoClient
   * @param {sanitizeData} sanitizeData
   */
  constructor(mongoClient, { sanitize, unsanitize }) {
    this.mongoClient = mongoClient.collection('dapContracts');
    this.sanitize = sanitize;
    this.unsanitize = unsanitize;
  }

  /**
   * Find DapContract by dapId
   *
   * @param {string} dapId
   * @returns {Promise<DapContract>}
   */
  async find(dapId) {
    const result = await this.mongoClient.findOne({ _id: dapId });
    const dapContractData = this.unsanitize(result || {});
    return new DapContract(
      dapContractData.dapId,
      dapContractData.dapName,
      dapContractData.packetHash,
      dapContractData.schema,
    );
  }

  /**
   * Store DapContract entity
   *
   * @param {DapContract} dapContract
   * @returns {Promise}
   */
  async store(dapContract) {
    const dapContractData = dapContract.toJSON();

    return this.mongoClient.updateOne(
      { _id: dapContractData.dapId },
      { $set: this.sanitize(dapContractData) },
      { upsert: true },
    );
  }
}

module.exports = DapContractMongoDbRepository;
