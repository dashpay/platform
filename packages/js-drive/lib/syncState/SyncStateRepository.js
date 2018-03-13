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
   * @param {STHeadersReaderState} state
   * @return {Promise}
   */
  async store(state) {
    return this.getCollection().updateOne(
      SyncStateRepository.mongoDbCondition,
      { $set: { blocks: state.getBlocks() } },
      { upsert: true },
    );
  }

  /**
   * Fetch synced blocks
   *
   * @return {Promise<STHeadersReaderState>}
   */
  async populate(state) {
    const { blocks } = await this.getCollection().findOne(SyncStateRepository.mongoDbCondition);

    state.setBlocks(blocks);

    return state;
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
