const SyncState = require('../SyncState');

class SyncStateRepository {
  /**
   * @param {Db} mongoDb
   */
  constructor(mongoDb) {
    this.mongoDb = mongoDb;
  }

  /**
   * Store synced blocks
   *
   * @param {SyncState} state
   * @return {Promise}
   */
  async store(state) {
    return this.getCollection().updateOne(
      SyncStateRepository.mongoDbCondition,
      { $set: state.toJSON() },
      { upsert: true },
    );
  }

  /**
   * Fetch synced blocks
   *
   * @return {Promise<SyncState>}
   */
  async fetch() {
    const stateData = await this.getCollection().findOne(SyncStateRepository.mongoDbCondition);

    let blocks = [];
    let lastSyncAt = null;
    if (stateData) {
      ({ blocks, lastSyncAt } = stateData);
    }

    return new SyncState(blocks, lastSyncAt);
  }

  /**
   * @private
   * @return {Collection}
   */
  getCollection() {
    return this.mongoDb.collection('syncState');
  }
}

SyncStateRepository.mongoDbCondition = { _id: 'state' };

module.exports = SyncStateRepository;
