require('dotenv-expand')(require('dotenv').config());

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

(async function main() {
  const rpcClient = new RpcClient({
    protocol: 'http',
    host: process.env.DASHCORE_JSON_RPC_HOST,
    port: process.env.DASHCORE_JSON_RPC_PORT,
    user: process.env.DASHCORE_JSON_RPC_USER,
    pass: process.env.DASHCORE_JSON_RPC_PASS,
  });

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

  // Initialize API methods
  const rpcMethods = {
    [addSTPacketMethod.name]: wrapToErrorHandler(addSTPacketMethod),
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

