const ReplicaSetInitError = require('./errors/ReplicaSetInitError');

async function checkReplicaSetInit(db) {
  const status = await db
    .admin()
    .command({ replSetGetStatus: 1 });

  if (!status) {
    throw new ReplicaSetInitError('Replica set status is empty', status);
  }

  if (!status.members) {
    throw new ReplicaSetInitError('Replica set have no members', status);
  }

  if (status.members[0].stateStr !== 'PRIMARY') {
    throw new ReplicaSetInitError('Replica set member is not in PRIMARY state', status);
  }
}

module.exports = checkReplicaSetInit;
