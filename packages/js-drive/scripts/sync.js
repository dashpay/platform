require('dotenv').config();

const zmq = require('zeromq');
const IpfsAPI = require('ipfs-api');
const RpcClient = require('bitcoind-rpc-dash/promise');
const { MongoClient } = require('mongodb');

const SyncStateRepository = require('../lib/sync/state/repository/SyncStateRepository');
const RpcBlockIterator = require('../lib/blockchain/iterator/RpcBlockIterator');
const StateTransitionHeaderIterator = require('../lib/blockchain/iterator/StateTransitionHeaderIterator');
const STHeadersReaderState = require('../lib/blockchain/reader/STHeadersReaderState');
const STHeadersReader = require('../lib/blockchain/reader/STHeadersReader');

const attachPinSTPacketHandler = require('../lib/storage/attachPinSTPacketHandler');
const attachStoreSyncStateHandler = require('../lib/sync/state/attachStoreSyncStateHandler');
const errorHandler = require('../lib/util/errorHandler');

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

  const mongoClient = await MongoClient.connect(process.env.STORAGE_MONGODB_URL);
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

  attachPinSTPacketHandler(stHeaderReader, ipfsAPI);
  attachStoreSyncStateHandler(stHeaderReader, syncState, syncStateRepository);

  await stHeaderReader.read();

  // Sync arriving ST packets
  const zmqSocket = zmq.createSocket('sub');
  zmqSocket.connect(process.env.DASHCORE_ZMQ_PUB_HASHBLOCK);

  let inSync = true;
  zmqSocket.on('message', (async () => {
    if (inSync) {
      return;
    }

    // Start sync from the last synced block + 1
    blockIterator.setBlockHeight(blockIterator.getBlockHeight() + 1);
    stHeaderIterator.reset(false);

    await stHeaderReader.read();

    inSync = false;
  }).catch(errorHandler));

  zmqSocket.subscribe('hashblock');
}()).catch(errorHandler);
