const DapObject = require('./DapObject');
const Reference = require('../Reference');

class DapObjectMongoDbRepository {
  /**
   * @param {Db} mongoClient
   */
  constructor(mongoClient) {
    this.mongoClient = mongoClient.collection('dapObjects');
  }

  /**
   * Find DapObject by id
   *
   * @param {string} id
   * @returns {Promise<DapObject>}
   */
  async find(id) {
    const result = await this.mongoClient.findOne({ _id: id });
    const dapObject = result || {};
    dapObject.object = dapObject.object || {};
    const referenceData = dapObject.reference || {};
    const reference = new Reference(
      referenceData.blockHash,
      referenceData.blockHeight,
      referenceData.stHeaderHash,
      referenceData.stPacketHash,
    );
    return new DapObject(dapObject.object, reference);
  }

  /**
   * Store DapObject entity
   *
   * @param {DapObject} dapObject
   * @returns {Promise}
   */
  store(dapObject) {
    return this.mongoClient.updateOne(
      { _id: dapObject.toJSON().id },
      { $set: dapObject.toJSON() },
      { upsert: true },
    );
  }

  /**
   * Delete DapObject entity
   *
   * @param dapObject
   * @returns {Promise}
   */
  async delete(dapObject) {
    return this.mongoClient.deleteOne({ _id: dapObject.toJSON().id });
  }
}

module.exports = DapObjectMongoDbRepository;
