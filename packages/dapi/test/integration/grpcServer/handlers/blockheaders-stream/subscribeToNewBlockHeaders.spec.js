const { BlockHeader, Block, ChainLock } = require('@dashevo/dashcore-lib');
const sinon = require('sinon');
const ZmqClient = require('../../../../../lib/externalApis/dashcore/ZmqClient');
const dashCoreRpcClient = require('../../../../../lib/externalApis/dashcore/rpc');

const subscribeToNewBlockHeaders = require('../../../../../lib/grpcServer/handlers/blockheaders-stream/subscribeToNewBlockHeaders');
const ProcessMediator = require('../../../../../lib/grpcServer/handlers/blockheaders-stream/ProcessMediator');

describe('subscribeToNewBlockHeaders', () => {
  let mediator;
  let zmqClient;
  const headers = {};

  beforeEach(async () => {
    const blockHeaderOne = new BlockHeader({
      version: 536870913,
      prevHash: '0000000000000000000000000000000000000000000000000000000000000000',
      merkleRoot: 'c4970326400177ce67ec582425a698b85ae03cae2b0d168e87eed697f1388e4b',
      time: 1507208925,
      timestamp: 1507208645,
      bits: 0,
      nonce: 1449878271,
    });

    const blockOne = new Block({
      header: blockHeaderOne.toObject(),
      transactions: [],
    });

    const blockHeaderTwo = new BlockHeader({
      version: 536870913,
      prevHash: blockOne.hash,
      merkleRoot: 'c4970326400177ce67ec582425a698b85ae03cae2b0d168e87eed697f1388e4c',
      time: 1507208926,
      timestamp: 1507208646,
      bits: 0,
      nonce: 1449878272,
    });

    const blockTwo = new Block({
      header: blockHeaderTwo.toObject(),
      transactions: [],
    });

    const blockHeaderThree = new BlockHeader({
      version: 536870913,
      prevHash: blockTwo.hash,
      merkleRoot: 'c4970326400177ce67ec582425a698b85ae03cae2b0d168e87eed697f1388e4d',
      time: 1507208927,
      timestamp: 1507208647,
      bits: 0,
      nonce: 1449878273,
    });

    headers[blockHeaderOne.hash] = blockHeaderOne;
    headers[blockHeaderTwo.hash] = blockHeaderTwo;
    headers[blockHeaderThree.hash] = blockHeaderThree;

    mediator = new ProcessMediator();
    sinon.stub(dashCoreRpcClient, 'getBlockHeader')
      .callsFake(async (hash) => headers[hash].toBuffer());

    zmqClient = new ZmqClient();
    sinon.stub(zmqClient.subscriberSocket, 'connect')
      .callsFake(() => {
        zmqClient.subscriberSocket.emit('connect');
      });
    await zmqClient.start();
  });

  afterEach(() => {
    // sinon.restore();
  });

  it('should add blocks and latest chain lock in cache and send them back when historical data is sent', async () => {
    const receivedHeaders = {};
    let latestChainLock = null;

    mediator.on(ProcessMediator.EVENTS.BLOCK_HEADERS, (blockHeaders) => {
      blockHeaders.forEach((header) => {
        receivedHeaders[header.hash] = header;
      });
    });

    mediator.on(ProcessMediator.EVENTS.CHAIN_LOCK, (chainLock) => {
      latestChainLock = chainLock;
    });

    subscribeToNewBlockHeaders(
      mediator,
      zmqClient,
      dashCoreRpcClient,
    );

    const hashes = Object.keys(headers);
    zmqClient.subscriberSocket.emit('message', zmqClient.topics.hashblock, Buffer.from(hashes[0], 'hex'));
    zmqClient.subscriberSocket.emit('message', zmqClient.topics.hashblock, Buffer.from(hashes[1], 'hex'));
    zmqClient.subscriberSocket.emit('message', zmqClient.topics.hashblock, Buffer.from(hashes[2], 'hex'));

    const chainLockOne = new ChainLock({
      height: 2,
      signature: Buffer.alloc(32).fill(1),
      blockHash: Buffer.alloc(32).fill(2),
    });

    const chainLockTwo = new ChainLock({
      height: 3,
      signature: Buffer.alloc(32).fill(3),
      blockHash: Buffer.alloc(32).fill(4),
    });

    zmqClient.subscriberSocket.emit('message', zmqClient.topics.rawchainlock, chainLockOne.toBuffer());
    zmqClient.subscriberSocket.emit('message', zmqClient.topics.rawchainlock, chainLockTwo.toBuffer());

    mediator.emit(ProcessMediator.EVENTS.HISTORICAL_DATA_SENT);

    await new Promise((resolve) => setImmediate(resolve));

    expect(receivedHeaders).to.deep.equal(headers);
    expect(latestChainLock).to.deep.equal(chainLockTwo);

    mediator.emit(ProcessMediator.EVENTS.CLIENT_DISCONNECTED);
  });
});
