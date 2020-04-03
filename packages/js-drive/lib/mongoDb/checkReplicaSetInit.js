const ReplicaSetInitError = require('./errors/ReplicaSetInitError');

/**
 *
 * @param {MongoClient} documentMongoDBClient
 * @return {Promise<void>}
 */
async function checkReplicaSetInit(documentMongoDBClient) {
  const status = await documentMongoDBClient.db('test')
    .admin()
    .command({ replSetGetStatus: 1 });

  if (!status) {
    throw new ReplicaSetInitError('Replica set status is empty', status);
  }

  if (!status.members || !status.members[0]) {
    throw new ReplicaSetInitError('Replica set have no members', status);
  }

  if (status.members[0].stateStr !== 'PRIMARY') {
    throw new ReplicaSetInitError('Replica set member is not in PRIMARY state', status);
  }
}

module.exports = checkReplicaSetInit;
