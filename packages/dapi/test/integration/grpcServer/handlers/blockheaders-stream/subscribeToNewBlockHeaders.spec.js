const { BlockHeader, Block, ChainLock } = require('@dashevo/dashcore-lib');
const ZmqClient = require('../../../../../lib/externalApis/dashcore/ZmqClient');
const dashCoreRpcClient = require('../../../../../lib/externalApis/dashcore/rpc');

const subscribeToNewBlockHeaders = require('../../../../../lib/grpcServer/handlers/blockheaders-stream/subscribeToNewBlockHeaders');
const ChainDataProvider = require('../../../../../lib/chainDataProvider/ChainDataProvider');
const BlockHeadersCache = require('../../../../../lib/chainDataProvider/BlockHeadersCache');
const { NEW_BLOCK_HEADERS_PROPAGATE_INTERVAL } = require('../../../../../lib/grpcServer/handlers/blockheaders-stream/constants');
const ProcessMediator = require('../../../../../lib/grpcServer/handlers/blockheaders-stream/ProcessMediator');
const wait = require('../../../../../lib/utils/wait');

describe('subscribeToNewBlockHeaders', () => {
  let mediator;
  let zmqClient;
  let chainDataProvider;
  let blockHeadersCache;
  let block;

  const blockHeaders = {};
  const chainLocks = {};

  beforeEach(async function beforeEach() {
    mediator = new ProcessMediator();

    this.sinon.stub(dashCoreRpcClient, 'getBlockHeader')
      .callsFake(async (hash) => blockHeaders[hash].toBuffer().toString('hex'));

    this.sinon.stub(dashCoreRpcClient, 'getBestChainLock').resolves({
      height: 1,
      signature: Buffer.from('fakeSig'),
      blockHash: Buffer.from('fakeHash'),
    });

    zmqClient = new ZmqClient();

    blockHeadersCache = new BlockHeadersCache();

    chainDataProvider = new ChainDataProvider(dashCoreRpcClient, zmqClient, blockHeadersCache);
    await chainDataProvider.init();

    dashCoreRpcClient.getBlockHeader.resetHistory();

    this.sinon.stub(zmqClient.subscriberSocket, 'connect')
      .callsFake(() => {
        zmqClient.subscriberSocket.emit('connect');
      });

    await zmqClient.start();

    block = new Block({
      header: {
        hash: '000000c546f0fdf0e20432a309e64ed75f05a6fdbb503bee46c813af6d4ef46d',
        version: 536870912,
        prevHash: '00000063859f5d58228bedbba96485e18c6aee5a55f72cb6eccbb00ffcb00afd',
        merkleRoot: '22b9b6dde8516991186f77687e49b4c09eb96e5d2adebdf1e0ae9a01251c13ee',
        time: 1608793053,
        bits: 503445090,
        nonce: 35025,
      },
      transactions: [
        {
          hash: '22b9b6dde8516991186f77687e49b4c09eb96e5d2adebdf1e0ae9a01251c13ee',
          version: 3,
          inputs: [
            {
              prevTxId: '0000000000000000000000000000000000000000000000000000000000000000',
              outputIndex: 4294967295,
              sequenceNumber: 4294967295,
              script: '024c0f010b',
            },
          ],
          outputs: [
            {
              satoshis: 20000000000,
              script: '76a91416b93a3b9168a20605cc3cda62f6135a3baa531a88ac',
            },
            {
              satoshis: 30000000000,
              script: '76a91416b93a3b9168a20605cc3cda62f6135a3baa531a88ac',
            },
          ],
          nLockTime: 0,
          type: 5,
          extraPayload: '02004c0f00003d8e273bf286d48ccba5a87b5adf332ed070a15e4e2d81eeb9ff685373be5656961e0b73ea855fdac9cc530782a7f0a22d25d1eaab4b2068efa647e9da0915d0',
        },
      ],
    });

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

    blockHeaders[blockHeaderOne.hash] = blockHeaderOne;
    blockHeaders[blockHeaderTwo.hash] = blockHeaderTwo;
    blockHeaders[blockHeaderThree.hash] = blockHeaderThree;

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

    const chainLockThree = new ChainLock({
      height: 4,
      signature: Buffer.alloc(32).fill(5),
      blockHash: Buffer.alloc(32).fill(6),
    });

    chainLocks[chainLockOne.height] = chainLockOne;
    chainLocks[chainLockTwo.height] = chainLockTwo;
    chainLocks[chainLockThree.height] = chainLockThree;
  });

  it('should add blocks and latest chain lock in cache and send them back when historical data is sent', async () => {
    const receivedHeaders = {};
    let latestChainLock = null;

    mediator.on(ProcessMediator.EVENTS.BLOCK_HEADERS, (headers) => {
      headers.forEach((header) => {
        receivedHeaders[header.hash] = header;
      });
    });

    mediator.on(ProcessMediator.EVENTS.CHAIN_LOCK, (chainLock) => {
      latestChainLock = chainLock;
    });

    subscribeToNewBlockHeaders(
      mediator,
      chainDataProvider,
    );

    const hashes = Object.keys(blockHeaders);
    zmqClient.subscriberSocket.emit('message', zmqClient.topics.hashblock, Buffer.from(hashes[0], 'hex'));
    zmqClient.subscriberSocket.emit('message', zmqClient.topics.hashblock, Buffer.from(hashes[1], 'hex'));
    zmqClient.subscriberSocket.emit('message', zmqClient.topics.hashblock, Buffer.from(hashes[2], 'hex'));

    const locksHeights = Object.keys(chainLocks);
    zmqClient.subscriberSocket.emit(
      'message',
      zmqClient.topics.rawchainlocksig,
      Buffer.concat([block.toBuffer(), chainLocks[locksHeights[0]].toBuffer()]),
    );
    zmqClient.subscriberSocket.emit(
      'message',
      zmqClient.topics.rawchainlocksig,
      Buffer.concat([block.toBuffer(), chainLocks[locksHeights[1]].toBuffer()]),
    );

    mediator.emit(ProcessMediator.EVENTS.HISTORICAL_DATA_SENT);

    await new Promise((resolve) => setImmediate(resolve));
    mediator.emit(ProcessMediator.EVENTS.CLIENT_DISCONNECTED);

    expect(receivedHeaders).to.deep.equal(blockHeaders);
    expect(latestChainLock).to.deep.equal(chainLocks[locksHeights[1]]);
  });

  it('should remove historical data from cache and send only data that is left', async () => {
    const receivedHeaders = {};

    mediator.on(ProcessMediator.EVENTS.BLOCK_HEADERS, (headers) => {
      headers.forEach((header) => {
        receivedHeaders[header.hash] = header;
      });
    });

    subscribeToNewBlockHeaders(
      mediator,
      chainDataProvider,
    );

    const hashes = Object.keys(blockHeaders);
    zmqClient.subscriberSocket.emit('message', zmqClient.topics.hashblock, Buffer.from(hashes[0], 'hex'));
    zmqClient.subscriberSocket.emit('message', zmqClient.topics.hashblock, Buffer.from(hashes[1], 'hex'));
    zmqClient.subscriberSocket.emit('message', zmqClient.topics.hashblock, Buffer.from(hashes[2], 'hex'));

    mediator.emit(ProcessMediator.EVENTS.HISTORICAL_BLOCK_HEADERS_SENT, [hashes[0]]);

    mediator.emit(ProcessMediator.EVENTS.HISTORICAL_DATA_SENT);

    await new Promise((resolve) => setImmediate(resolve));
    mediator.emit(ProcessMediator.EVENTS.CLIENT_DISCONNECTED);

    const expectedHeaders = { ...blockHeaders };
    delete expectedHeaders[hashes[0]];
    expect(receivedHeaders).to.deep.equal(expectedHeaders);
  });

  it('should send fresh chain locks', async () => {
    const receivedChainLocks = {};

    mediator.on(ProcessMediator.EVENTS.CHAIN_LOCK, (chainLock) => {
      receivedChainLocks[chainLock.height] = chainLock;
    });

    subscribeToNewBlockHeaders(
      mediator,
      chainDataProvider,
    );

    const locksHeights = Object.keys(chainLocks);
    zmqClient.subscriberSocket.emit(
      'message',
      zmqClient.topics.rawchainlocksig,
      Buffer.concat([block.toBuffer(), chainLocks[locksHeights[0]].toBuffer()]),
    );
    mediator.emit(ProcessMediator.EVENTS.HISTORICAL_DATA_SENT);
    zmqClient.subscriberSocket.emit(
      'message',
      zmqClient.topics.rawchainlocksig,
      Buffer.concat([block.toBuffer(),
        chainLocks[locksHeights[1]].toBuffer()]),
    );
    zmqClient.subscriberSocket.emit(
      'message',
      zmqClient.topics.rawchainlocksig,
      Buffer.concat([block.toBuffer(), chainLocks[locksHeights[2]].toBuffer()]),
    );
    await wait(NEW_BLOCK_HEADERS_PROPAGATE_INTERVAL + 100);
    mediator.emit(ProcessMediator.EVENTS.CLIENT_DISCONNECTED);
    const expectedChainLocks = { ...chainLocks };
    delete expectedChainLocks[locksHeights[1]];
    expect(receivedChainLocks).to.deep.equal(expectedChainLocks);
  });

  it('should use cache when historical data is sent', async () => {
    const receivedHeaders = {};

    mediator.on(ProcessMediator.EVENTS.BLOCK_HEADERS, (headers) => {
      headers.forEach((header) => {
        receivedHeaders[header.hash] = header;
      });
    });

    subscribeToNewBlockHeaders(
      mediator,
      chainDataProvider,
    );

    const hashes = Object.keys(blockHeaders);
    zmqClient.subscriberSocket.emit('message', zmqClient.topics.hashblock, Buffer.from(hashes[0], 'hex'));
    zmqClient.subscriberSocket.emit('message', zmqClient.topics.hashblock, Buffer.from(hashes[1], 'hex'));
    zmqClient.subscriberSocket.emit('message', zmqClient.topics.hashblock, Buffer.from(hashes[2], 'hex'));

    const locksHeights = Object.keys(chainLocks);
    zmqClient.subscriberSocket.emit(
      'message',
      zmqClient.topics.rawchainlocksig,
      Buffer.concat([block.toBuffer(), chainLocks[locksHeights[0]].toBuffer()]),
    );
    zmqClient.subscriberSocket.emit(
      'message',
      zmqClient.topics.rawchainlocksig,
      Buffer.concat([block.toBuffer(), chainLocks[locksHeights[1]].toBuffer()]),
    );

    mediator.emit(ProcessMediator.EVENTS.HISTORICAL_DATA_SENT);

    await new Promise((resolve) => setImmediate(resolve));
    mediator.emit(ProcessMediator.EVENTS.CLIENT_DISCONNECTED);

    expect(dashCoreRpcClient.getBlockHeader.callCount).to.be.equal(3);
    dashCoreRpcClient.getBlockHeader.resetHistory();

    subscribeToNewBlockHeaders(
      mediator,
      chainDataProvider,
    );

    zmqClient.subscriberSocket.emit('message', zmqClient.topics.hashblock, Buffer.from(hashes[0], 'hex'));
    zmqClient.subscriberSocket.emit('message', zmqClient.topics.hashblock, Buffer.from(hashes[1], 'hex'));
    zmqClient.subscriberSocket.emit('message', zmqClient.topics.hashblock, Buffer.from(hashes[2], 'hex'));

    zmqClient.subscriberSocket.emit(
      'message',
      zmqClient.topics.rawchainlocksig,
      Buffer.concat([block.toBuffer(), chainLocks[locksHeights[0]].toBuffer()]),
    );
    zmqClient.subscriberSocket.emit(
      'message',
      zmqClient.topics.rawchainlocksig,
      Buffer.concat([block.toBuffer(), chainLocks[locksHeights[1]].toBuffer()]),
    );

    mediator.emit(ProcessMediator.EVENTS.HISTORICAL_DATA_SENT);

    expect(dashCoreRpcClient.getBlockHeader.callCount).to.be.equal(0);
  });
});
