require('dotenv-expand')(require('dotenv-safe').config());

const zmq = require('zeromq');
const IpfsAPI = require('ipfs-api');
const RpcClient = require('bitcoind-rpc-dash/promise');
const { MongoClient } = require('mongodb');

const SyncStateRepository = require('../lib/sync/state/repository/SyncStateRepository');
const RpcBlockIterator = require('../lib/blockchain/iterator/RpcBlockIterator');
const StateTransitionHeaderIterator = require('../lib/blockchain/iterator/StateTransitionHeaderIterator');
const STHeadersReaderState = require('../lib/blockchain/reader/STHeadersReaderState');
const STHeadersReader = require('../lib/blockchain/reader/STHeadersReader');
const sanitizeData = require('../lib/mongoDb/sanitizeData');
const DapContractMongoDbRepository = require('../lib/stateView/dapContract/DapContractMongoDbRepository');
const DapObjectMongoDbRepository = require('../lib/stateView/dapObject/DapObjectMongoDbRepository');
const createDapObjectMongoDbRepositoryFactory = require('../lib/stateView/dapObject/createDapObjectMongoDbRepositoryFactory');
const updateDapContractFactory = require('../lib/stateView/dapContract/updateDapContractFactory');
const updateDapObjectFactory = require('../lib/stateView/dapObject/updateDapObjectFactory');
const applyStateTransitionFactory = require('../lib/stateView/applyStateTransitionFactory');

const cleanDashDriveFactory = require('../lib/sync/cleanDashDriveFactory');
const unpinAllIpfsPacketsFactory = require('../lib/storage/ipfs/unpinAllIpfsPacketsFactory');
const dropMongoDatabasesWithPrefixFactory = require('../lib/mongoDb/dropMongoDatabasesWithPrefixFactory');

const attachStorageHandlers = require('../lib/storage/attachStorageHandlers');
const attachSyncHandlers = require('../lib/sync/state/attachSyncHandlers');
const attachStateViewHandlers = require('../lib/stateView/attachStateViewHandlers');
const errorHandler = require('../lib/util/errorHandler');

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

  const blockIterator = new RpcBlockIterator(
    rpcClient,
    parseInt(process.env.SYNC_EVO_START_BLOCK_HEIGHT, 10),
  );

  const stHeaderIterator = new StateTransitionHeaderIterator(blockIterator, rpcClient);

  const mongoClient = await MongoClient.connect(
    process.env.STORAGE_MONGODB_URL,
    { useNewUrlParser: true },
  );
  const mongoDb = mongoClient.db(process.env.STORAGE_MONGODB_DB);
  const syncStateRepository = new SyncStateRepository(mongoDb);
  const syncState = await syncStateRepository.fetch();

  // TODO: Parse variable to int if present
  const stHeadersReaderState = new STHeadersReaderState(
    syncState.getBlocks(),
    process.env.SYNC_STATE_BLOCKS_LIMIT,
  );

  const stHeaderReader = new STHeadersReader(stHeaderIterator, stHeadersReaderState);

  const ipfsAPI = new IpfsAPI(process.env.STORAGE_IPFS_MULTIADDR);

  const unpinAllIpfsPackets = unpinAllIpfsPacketsFactory(ipfsAPI);
  const dropMongoDatabasesWithPrefix = dropMongoDatabasesWithPrefixFactory(mongoClient);
  const cleanDashDrive = cleanDashDriveFactory(unpinAllIpfsPackets, dropMongoDatabasesWithPrefix);

  attachStorageHandlers(stHeaderReader, ipfsAPI, unpinAllIpfsPackets);
  attachSyncHandlers(stHeaderReader, syncState, syncStateRepository);
  const dapContractMongoDbRepository = new DapContractMongoDbRepository(mongoDb, sanitizeData);
  const createDapObjectMongoDbRepository = createDapObjectMongoDbRepositoryFactory(
    mongoClient,
    DapObjectMongoDbRepository,
  );
  const updateDapContract = updateDapContractFactory(dapContractMongoDbRepository);
  const updateDapObject = updateDapObjectFactory(createDapObjectMongoDbRepository);
  const applyStateTransition = applyStateTransitionFactory(
    ipfsAPI,
    updateDapContract,
    updateDapObject,
  );
  attachStateViewHandlers(stHeaderReader, applyStateTransition, dropMongoDatabasesWithPrefix);

  const isDashCoreRunning = isDashCoreRunningFactory(rpcClient);

  let isFirstSyncCompleted = false;
  let isInSync = false;

  async function resetDashDrive() {
    await cleanDashDrive(process.env.MONGODB_DB_PREFIX);
    stHeadersReaderState.clear();
    stHeaderIterator.reset(false);
    blockIterator.setBlockHeight(1);
    syncState.setBlocks([]);
    syncState.setLastSyncAt(null);
  }

  /**
   * @param {Buffer} [sinceBlockHash]
   * @returns {Promise<void>}
   */
  async function sync(sinceBlockHash = undefined) {
    if (isInSync) {
      return;
    }

    isInSync = true;

    try {
      // Start sync from the last synced block + 1
      let height = blockIterator.getBlockHeight();
      if (isFirstSyncCompleted) {
        height += 1;
      }

      // Reset height to the current block's height
      if (sinceBlockHash) {
        const { result: { height: blockHeight } } = await rpcClient.getBlock(sinceBlockHash.toString('hex'));
        if (blockHeight < height) {
          height = blockHeight;
        }
      }

      blockIterator.setBlockHeight(height);
      stHeaderIterator.reset(false);

      try {
        await stHeaderReader.read();
      } catch (error) {
        if (error.message !== 'Block height out of range') {
          throw error;
        }

        if (!syncState.isEmpty()) {
          await resetDashDrive();

          isInSync = false;
          isFirstSyncCompleted = false;

          await sync();
          return;
        }
      }

      isFirstSyncCompleted = true;
      isInSync = false;
    } catch (e) {
      isInSync = false;

      throw e;
    }
  }

  const isRunning = await isDashCoreRunning();
  if (!isRunning) {
    throw new DashCoreIsNotRunningError();
  }

  await sync();

  // Sync arriving ST packets
  const zmqSocket = zmq.createSocket('sub');
  zmqSocket.connect(process.env.DASHCORE_ZMQ_PUB_HASHBLOCK);

  zmqSocket.on('message', (topic, blockHash) => {
    sync(blockHash).catch((error) => {
      isInSync = false;
      errorHandler(error);
    });
  });

  zmqSocket.subscribe('hashblock');
}()).catch(errorHandler);
