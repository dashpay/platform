const EventEmitter = require('events');
const ZMQClient = require('@dashevo/dashd-zmq');
const LatestCoreChainLock = require('../../../lib/core/LatestCoreChainLock');
const waitForCoreChainLockSyncFactory = require('../../../lib/core/waitForCoreChainLockSyncFactory');

describe('waitForCoreChainLockSyncFactory', () => {
  let waitForCoreChainLockHandler;
  let coreRpcClientMock;
  let coreZMQClientMock;
  let latestCoreChainLock;
  let chainLock;

  beforeEach(function beforeEach() {
    chainLock = {
      height: 84202,
      blockHash: '0000000007e0a65b763c0a4fb2274ff757abdbd19c9efe9de189f5828c70a5f4',
      signature: '0a43f1c3e5b3e8dbd670bca8d437dc25572f72d8e1e9be673e9ebbb606570307c3e5f5d073f7beb209dd7e0b8f96c751060ab3a7fb69a71d5ccab697b8cfa5a91038a6fecf76b7a827d75d17f01496302942aa5e2c7f4a48246efc8d3941bf6c',
    };

    latestCoreChainLock = new LatestCoreChainLock();
    coreRpcClientMock = {
      getBestChainLock: this.sinon.stub().resolves({
        result: chainLock,
        error: null,
        id: 5,
      }),
      getBlock: this.sinon.stub(),
    };
    coreZMQClientMock = new EventEmitter();
    coreZMQClientMock.subscribe = this.sinon.stub();

    const loggerMock = {
      debug: this.sinon.stub(),
      info: this.sinon.stub(),
      trace: this.sinon.stub(),
      error: this.sinon.stub(),
    };

    waitForCoreChainLockHandler = waitForCoreChainLockSyncFactory(
      coreZMQClientMock,
      coreRpcClientMock,
      latestCoreChainLock,
      loggerMock,
    );
  });

  it('should wait for chainlock to be synced', async () => {
    expect(latestCoreChainLock.chainLock).to.equal(undefined);

    await waitForCoreChainLockHandler();

    expect(latestCoreChainLock.chainLock.toJSON()).to.deep.equal(chainLock);

    expect(coreZMQClientMock.subscribe).to.be.calledTwice();
    expect(coreZMQClientMock.subscribe).to.be.calledWith(ZMQClient.TOPICS.rawchainlock);
    expect(coreZMQClientMock.subscribe).to.be.calledWith(ZMQClient.TOPICS.hashblock);
    expect(coreRpcClientMock.getBestChainLock).to.be.calledOnce();
  });

  it('should handle when no chainlock is found via RPC', (done) => {
    expect(latestCoreChainLock.chainLock).to.equal(undefined);

    const err = new Error();
    err.code = -32603;
    err.message = 'Chainlock not found';

    coreRpcClientMock.getBestChainLock.throws(err);

    waitForCoreChainLockHandler()
      .then(() => {
        expect(latestCoreChainLock.chainLock.toJSON()).to.deep.equal(chainLock);

        expect(coreZMQClientMock.subscribe).to.be.calledTwice();
        expect(coreZMQClientMock.subscribe).to.be.calledWith(ZMQClient.TOPICS.rawchainlock);
        expect(coreZMQClientMock.subscribe).to.be.calledWith(ZMQClient.TOPICS.hashblock);
        expect(coreRpcClientMock.getBestChainLock).to.be.calledOnce();
        done();
      });

    setImmediate(() => {
      coreZMQClientMock.emit(ZMQClient.TOPICS.rawchainlock, {
        height: 84202,
        blockHash: '0000000007e0a65b763c0a4fb2274ff757abdbd19c9efe9de189f5828c70a5f4',
        signature: '0a43f1c3e5b3e8dbd670bca8d437dc25572f72d8e1e9be673e9ebbb606570307c3e5f5d073f7beb209dd7e0b8f96c751060ab3a7fb69a71d5ccab697b8cfa5a91038a6fecf76b7a827d75d17f01496302942aa5e2c7f4a48246efc8d3941bf6c',
      });
    });

    setImmediate(() => {
      coreZMQClientMock.emit(ZMQClient.TOPICS.hashblock, '0000000007e0a65b763c0a4fb2274ff757abdbd19c9efe9de189f5828c70a5f4');
    });
  });
});
