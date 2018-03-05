/* eslint-disable no-console */

require('dotenv').config();

const zmq = require('zeromq');
const IpfsAPI = require('ipfs-api');
const RpcClient = require('bitcoind-rpc-dash');

const BlockIterator = require('../lib/blockchain/BlockIterator');
const StateTransitionHeaderIterator = require('../lib/blockchain/StateTransitionHeaderIterator');
const STHeadersIterationEventEmitter = require('../lib/blockchain/STHeadersIterationEventEmitter');
const attachPinSTPacketHandler = require('../lib/storage/attachPinSTPacketHandler');

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
  const blockIterator = new BlockIterator(rpcClient, process.env.EVO_GENESIS_BLOCK_HEIGHT);
  const stHeaderIterator = new StateTransitionHeaderIterator(blockIterator);

  const iterationEmitter = new STHeadersIterationEventEmitter(stHeaderIterator);

  attachPinSTPacketHandler(ipfsAPI, iterationEmitter);

  await iterationEmitter.iterate();

  // Sync arriving ST packets
  const zmqSocket = zmq.createSocket('sub');

  zmqSocket.connect(process.env.DASHCORE_ZMQ_PUB_RAWST);

  let inSync = true;
  zmqSocket.on('message', (async () => {
    if (inSync) {
      return;
    }

    await iterationEmitter.iterate();

    inSync = false;
  }).catch(handleError));

  zmqSocket.subscribe('zmqpubhashblock');
}

main().catch(handleError);
