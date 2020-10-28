const EventEmitter = require('events');
const ZMQClient = require('@dashevo/dashd-zmq');
const LatestCoreChainLock = require('../../../lib/core/LatestCoreChainLock');
const waitForCoreChainLockSyncFallbackFactory = require('../../../lib/core/waitForCoreChainLockSyncFallbackFactory');

describe('waitForCoreChainLockSyncFallbackFactory', () => {
  let waitForCoreChainLockSyncFallback;
  let coreRpcClientMock;
  let coreZMQClientMock;
  let latestCoreChainLock;
  let errorHandlerMock;
  let blockHash;
  let signature;
  let chainLock;
  let height;
  let block;

  beforeEach(function beforeEach() {
    height = 1;
    signature = Buffer.alloc(32).toString('hex');
    blockHash = '0000000007e0a65b763c0a4fb2274ff757abdbd19c9efe9de189f5828c70a5f4';

    chainLock = {
      height,
      blockHash,
      signature,
    };

    block = {
      hash: blockHash,
      confirmations: 1,
      size: 306,
      height,
      version: 1,
      versionHex: '00000001',
      merkleroot: 'e0028eb9648db56b1ac77cf090b99048a8007e2bb64b68f092c03c7f56a662c7',
      tx: [
        'e0028eb9648db56b1ac77cf090b99048a8007e2bb64b68f092c03c7f56a662c7',
      ],
      time: 1417713337,
      mediantime: 1417713337,
      nonce: 1096447,
      bits: '207fffff',
      difficulty: 4.656542373906925e-10,
      chainwork: '0000000000000000000000000000000000000000000000000000000000000002',
      nTx: 1,
      chainlock: false,
    };

    latestCoreChainLock = new LatestCoreChainLock();
    coreRpcClientMock = {
      getBestBlockHash: this.sinon.stub().resolves({
        result: blockHash,
        error: null,
        id: 5,
      }),
      getBlock: this.sinon.stub().resolves({
        result: block,
        error: null,
        id: 6,
      }),
    };
    coreZMQClientMock = new EventEmitter();
    coreZMQClientMock.connect = this.sinon.stub();
    coreZMQClientMock.subscribe = this.sinon.stub();

    errorHandlerMock = this.sinon.stub();

    const loggerMock = {
      debug: this.sinon.stub(),
      info: this.sinon.stub(),
      trace: this.sinon.stub(),
      error: this.sinon.stub(),
    };

    waitForCoreChainLockSyncFallback = waitForCoreChainLockSyncFallbackFactory(
      coreZMQClientMock,
      coreRpcClientMock,
      latestCoreChainLock,
      loggerMock,
      errorHandlerMock,
    );
  });

  it('should wait for the block', async () => {
    expect(latestCoreChainLock.chainLock).to.equal(undefined);

    await waitForCoreChainLockSyncFallback();

    expect(latestCoreChainLock.chainLock.toJSON()).to.deep.equal(chainLock);

    expect(coreZMQClientMock.subscribe).to.be.calledOnce();
    expect(coreZMQClientMock.subscribe).to.be.calledWith(ZMQClient.TOPICS.hashblock);
    expect(coreZMQClientMock.connect).to.be.calledOnce();
    expect(coreRpcClientMock.getBestBlockHash).to.be.calledOnce();
    expect(coreRpcClientMock.getBlock).to.be.calledOnceWithExactly(blockHash);
  });

  it('should handle when no blocks is found via RPC', (done) => {
    height = 0;
    block.height = 0;
    chainLock.height = 0;

    waitForCoreChainLockSyncFallback()
      .then(() => {
        expect(latestCoreChainLock.chainLock.toJSON()).to.deep.equal(chainLock);

        expect(coreZMQClientMock.subscribe).to.be.calledOnce();
        expect(coreZMQClientMock.subscribe).to.be.calledWith(ZMQClient.TOPICS.hashblock);
        expect(coreZMQClientMock.connect).to.be.calledOnce();
        expect(coreRpcClientMock.getBestBlockHash).to.be.calledOnce();
        expect(coreRpcClientMock.getBlock).to.be.calledTwice();
        done();
      });

    setImmediate(() => {
      coreZMQClientMock.emit(ZMQClient.TOPICS.hashblock, blockHash);
    });
  });

  it('should call errorHandler on end event', (done) => {
    height = 0;
    block.height = 0;
    chainLock.height = 0;

    const err = new Error();
    err.code = -32603;
    err.message = 'Block not found';

    waitForCoreChainLockSyncFallback()
      .then(() => {
        expect(latestCoreChainLock.chainLock.toJSON()).to.deep.equal(chainLock);

        expect(coreZMQClientMock.subscribe).to.be.calledOnce();
        expect(coreZMQClientMock.subscribe).to.be.calledWith(ZMQClient.TOPICS.hashblock);
        expect(coreZMQClientMock.connect).to.be.calledOnce();
        expect(coreRpcClientMock.getBestBlockHash).to.be.calledOnce();
        expect(coreRpcClientMock.getBlock).to.be.calledTwice();

        const error = new Error(`Lost connection with Core: ${err.message}`);

        expect(errorHandlerMock.getCall(0).args[0].message).to.equal(error.message);
        done();
      });

    setImmediate(() => {
      coreZMQClientMock.emit('end', err);
    });

    setImmediate(() => {
      coreZMQClientMock.emit(ZMQClient.TOPICS.hashblock, blockHash);
    });
  });
});
