require('dotenv-expand')(require('dotenv-safe').config());

const connect = require('connect');
const jayson = require('jayson/promise');

const IpfsAPI = require('ipfs-api');
const RpcClient = require('bitcoind-rpc-dash/promise');
const { MongoClient } = require('mongodb');

const SyncStateRepository = require('../lib/sync/state/repository/SyncStateRepository');
const SyncStateRepositoryChangeListener = require('../lib/sync/state/repository/SyncStateRepositoryChangeListener');

const isSynced = require('../lib/sync/isSynced');
const getCheckSyncStateHttpMiddleware = require('../lib/sync/getCheckSyncHttpMiddleware');
const errorHandler = require('../lib/util/errorHandler');

const wrapToErrorHandler = require('../lib/api/jsonRpc/wrapToErrorHandler');

const addSTPacketFactory = require('../lib/storage/ipfs/addSTPacketFactory');
const addSTPacketMethodFactory = require('../lib/api/methods/addSTPacketMethodFactory');

const DapObjectMongoDbRepository = require('../lib/stateView/dapObject/DapObjectMongoDbRepository');
const createDapObjectMongoDbRepositoryFactory = require('../lib/stateView/dapObject/createDapObjectMongoDbRepositoryFactory');
const fetchDapObjectsFactory = require('../lib/stateView/dapObject/fetchDapObjectsFactory');
const fetchDapObjectsMethodFactory = require('../lib/api/methods/fetchDapObjectsMethodFactory');

const isDashCoreRunningFactory = require('../lib/sync/isDashCoreRunningFactory');
const DashCoreIsNotRunningError = require('../lib/sync/DashCoreIsNotRunningError');


(async function main() {
  const rpcClient = new RpcClient({
    protocol: 'http',
    host: process.env.DASHCORE_JSON_RPC_HOST,
    port: process.env.DASHCORE_JSON_RPC_PORT,
    user: process.env.DASHCORE_JSON_RPC_USER,
    pass: process.env.DASHCORE_JSON_RPC_PASS,
  });

  const isDashCoreRunning = isDashCoreRunningFactory(rpcClient);
  const isRunning = await isDashCoreRunning(
    process.env.DASHCORE_RUNNING_CHECK_MAX_RETRIES,
    process.env.DASHCORE_RUNNING_CHECK_INTERVAL,
  );
  if (!isRunning) {
    throw new DashCoreIsNotRunningError();
  }

  const mongoClient = await MongoClient.connect(
    process.env.STORAGE_MONGODB_URL,
    { useNewUrlParser: true },
  );
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
    process.env.SYNC_CHAIN_CHECK_INTERVAL,
  );

  const ipfsAPI = new IpfsAPI(process.env.STORAGE_IPFS_MULTIADDR);
  const addSTPacket = addSTPacketFactory(ipfsAPI);
  const addSTPacketMethod = addSTPacketMethodFactory(addSTPacket);

  const createDapObjectMongoDbRepository = createDapObjectMongoDbRepositoryFactory(
    mongoClient,
    DapObjectMongoDbRepository,
  );
  const fetchDapObjects = fetchDapObjectsFactory(createDapObjectMongoDbRepository);
  const fetchDapObjectsMethod = fetchDapObjectsMethodFactory(fetchDapObjects);


  /**
   * Remove 'Method' Postfix
   *
   * Takes a function as an argument, returns the function's name
   * as a string without 'Method' as a postfix.
   *
   * @param {function} func Function that uses 'Method' postfix
   * @returns {string} String of function name without 'Method' postfix
   */
  function rmPostfix(func) {
    const funcName = func.name;
    return funcName.substr(0, funcName.length - 'Method'.length);
  }

  // Initialize API methods
  const rpcMethods = {
    [rmPostfix(addSTPacketMethod)]: wrapToErrorHandler(addSTPacketMethod),
    [rmPostfix(fetchDapObjectsMethod)]: wrapToErrorHandler(fetchDapObjectsMethod),
  };

  const rpc = jayson.server(rpcMethods);
  const server = connect();

  server.use(checkSyncState);
  server.use(rpc.middleware());

  server.listen(
    process.env.API_RPC_PORT,
    process.env.API_RPC_HOST || '0.0.0.0',
  );
}()).catch(errorHandler);

