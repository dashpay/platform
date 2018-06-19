const DapObject = require('./DapObject');

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
    return new DapObject(dapObject.id);
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
}

module.exports = DapObjectMongoDbRepository;
