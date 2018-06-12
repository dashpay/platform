const DapContract = require('./DapContract');

class DapContractMongoDbRepository {
  /**
   * @param {Db} mongoClient
   */
  constructor(mongoClient) {
    this.mongoClient = mongoClient.collection('dapContracts');
  }

  /**
   * Find DapContract by dapId
   *
   * @param {string} dapId
   * @returns {Promise<DapContract>}
   */
  async find(dapId) {
    const result = await this.mongoClient.findOne({ _id: dapId });
    const dapContractData = result || {};
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
    return this.mongoClient.updateOne(
      { _id: dapContract.toJSON().dapId },
      { $set: dapContract.toJSON() },
      { upsert: true },
    );
  }
}

module.exports = DapContractMongoDbRepository;
