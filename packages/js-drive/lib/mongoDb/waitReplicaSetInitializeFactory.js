const wait = require('../util/wait');

const ReplicaSetIsNotInitializedError = require('./errors/ReplicaSetIsNotInitializedError');

/**
 *
 * @param {connectToMongoDB} connectToDocumentMongoDB
 * @return {waitReplicaSetInitialize}
 */
function waitReplicaSetInitializeFactory(connectToDocumentMongoDB) {
  /**
   * Wait until mongoDB replica set to be initialized
   * @typedef waitReplicaSetInitialize
   * @param {function(number, number)} progressCallback
   * @return {Promise<void>}
   */
  async function waitReplicaSetInitialize(progressCallback) {
    let lastError;
    let isInitialized = false;
    let retries = 0;

    const maxRetries = 10;

    while (!isInitialized && retries < maxRetries) {
      try {
        const mongoClient = await connectToDocumentMongoDB();

        const status = await mongoClient.db('test')
          .admin()
          .command({ replSetGetStatus: 1 });

        isInitialized = status && status.members && status.members[0] && status.members[0].stateStr === 'PRIMARY';
      } catch (e) {
        // skip the error
        lastError = e;
      } finally {
        if (!isInitialized) {
          retries += 1;
          progressCallback(retries, maxRetries);

          await wait(1000);
        }
      }
    }

    if (!isInitialized) {
      throw new ReplicaSetIsNotInitializedError(lastError);
    }
  }

  return waitReplicaSetInitialize;
}

module.exports = waitReplicaSetInitializeFactory;
