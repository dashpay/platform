require('dotenv').config();

const connect = require('connect');
const jayson = require('jayson/promise');

const { MongoClient } = require('mongodb');
const RpcClient = require('@dashevo/bitcoind-rpc-dash/promise');

const SyncStateRepository = require('../lib/sync/state/repository/SyncStateRepository');
const SyncStateRepositoryChangeListener = require('../lib/sync/state/repository/SyncStateRepositoryChangeListener');

const isSynced = require('../lib/sync/isSynced');
const getCheckSyncStateHttpMiddleware = require('../lib/sync/getCheckSyncHttpMiddleware');
const errorHandler = require('../lib/util/errorHandler');

const rpcHandlers = require('../lib/api/rpc');

(async function main() {
  const rpcClient = new RpcClient({
    protocol: 'http',
    host: process.env.DASHCORE_JSON_RPC_HOST,
    port: process.env.DASHCORE_JSON_RPC_PORT,
    user: process.env.DASHCORE_JSON_RPC_USER,
    pass: process.env.DASHCORE_JSON_RPC_PASS,
  });

  const mongoClient = await MongoClient.connect(process.env.STORAGE_MONGODB_URL);
  const mongoDb = mongoClient.db(process.env.STORAGE_MONGODB_DB);
  const syncStateRepository = new SyncStateRepository(mongoDb);
  // TODO: Validate env variable (should be number > 0)
  const repositoryChangeListener = new SyncStateRepositoryChangeListener(
    syncStateRepository,
    process.env.SYNC_STATE_CHECK_INTERVAL * 1000,
  );

  const checkSyncState = getCheckSyncStateHttpMiddleware(
    isSynced,
    rpcClient,
    repositoryChangeListener,
  );

  const rpc = jayson.server(rpcHandlers);
  const server = connect();

  server.use(checkSyncState);
  server.use(rpc.middleware());

  server.listen(
    process.env.API_RPC_PORT,
    process.env.API_RPC_HOST || '0.0.0.0',
  );
}()).catch(errorHandler);

