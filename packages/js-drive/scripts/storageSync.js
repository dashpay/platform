/* eslint-disable no-console */

require('dotenv').config();

const zmq = require('zeromq');
const IpfsAPI = require('ipfs-api');
const RpcClient = require('bitcoind-rpc-dash');
const { MongoClient } = require('mongodb');

const SyncStateRepository = require('../lib/syncState/SyncStateRepository');
const BlockIterator = require('../lib/blockchain/BlockIterator');
const StateTransitionHeaderIterator = require('../lib/blockchain/StateTransitionHeaderIterator');
const STHeadersReaderState = require('../lib/blockchain/STHeadersReaderState');
const STHeadersReader = require('../lib/blockchain/STHeadersReader');

const attachPinSTPacketHandler = require('../lib/storage/attachPinSTPacketHandler');
const attachStoreSyncStateHandler = require('../lib/syncState/attachStoreSyncStateHandler');

const ipfsAPI = new IpfsAPI(process.env.STORAGE_IPFS_MULTIADDR);

function handleError(e) {
  console.error(e);
}

// Sync ST packets since genesis block
async function main() {
  const rpcClient = new RpcClient({
    protocol: 'http',
    host: process.env.DASHCORE_JSON_RPC_HOST,
    port: process.env.DASHCORE_JSON_RPC_PORT,
    user: process.env.DASHCORE_JSON_RPC_USER,
    pass: process.env.DASHCORE_JSON_RPC_PASS,
  });
  const blockIterator = new BlockIterator(rpcClient, process.env.SYNC_EVO_START_BLOCK_HEIGHT);
  const stHeaderIterator = new StateTransitionHeaderIterator(blockIterator);
  const stHeadersReaderState = new STHeadersReaderState([], process.env.SYNC_STATE_BLOCKS_LIMIT);

  const mongoClient = await MongoClient.connect(process.env.STORAGE_MONGODB_URL);
  const mongoDb = mongoClient.db(process.env.STORAGE_MONGODB_DB);
  const syncStateRepository = new SyncStateRepository(mongoDb);
  syncStateRepository.populate(stHeadersReaderState);

  const stHeaderReader = new STHeadersReader(stHeaderIterator, stHeadersReaderState);

  attachPinSTPacketHandler(stHeaderReader, ipfsAPI);
  attachStoreSyncStateHandler(stHeaderReader, syncStateRepository);

  await stHeaderReader.read();

  // Sync arriving ST packets
  const zmqSocket = zmq.createSocket('sub');
  zmqSocket.connect(process.env.DASHCORE_ZMQ_PUB_RAWST);

  let inSync = true;
  zmqSocket.on('message', (async () => {
    if (inSync) {
      return;
    }

    await stHeaderReader.read();

    inSync = false;
  }).catch(handleError));

  zmqSocket.subscribe('hashblock');
}

main().catch(handleError);
