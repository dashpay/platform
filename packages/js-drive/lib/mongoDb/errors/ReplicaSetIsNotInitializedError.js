class ReplicaSetIsNotInitializedError extends Error {
  /**
   * ReplicaSet is not initialized error
   * @param {Error} mongoDbError
   */
  constructor(mongoDbError) {
    super();

    this.name = this.constructor.name;
    this.message = `Replica set is not initialized: ${mongoDbError.message}`;
    this.mongoDbError = mongoDbError;

    Error.captureStackTrace(this, this.constructor);
  }

  /**
   * Get original mongoDB error
   * @return {Error}
   */
  getMongoDbError() {
    return this.mongoDbError;
  }
}

module.exports = ReplicaSetIsNotInitializedError;
