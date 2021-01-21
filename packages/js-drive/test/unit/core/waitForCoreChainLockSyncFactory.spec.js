const EventEmitter = require('events');
const ZMQClient = require('@dashevo/dashd-zmq');
const LatestCoreChainLock = require('../../../lib/core/LatestCoreChainLock');
const waitForCoreChainLockSyncFactory = require('../../../lib/core/waitForCoreChainLockSyncFactory');
const LoggerMock = require('../../../lib/test/mock/LoggerMock');

describe('waitForCoreChainLockSyncFactory', () => {
  let waitForCoreChainLockHandler;
  let coreRpcClientMock;
  let coreZMQClientMock;
  let latestCoreChainLock;
  let chainLock;
  let rawChainLockSigMessage;

  beforeEach(function beforeEach() {
    chainLock = {
      blockHash: '0000003df90e1cec3fea6bd17508f653cea093c536199e9d50a05bd69ee23b5d',
      height: 3887,
      signature: '1770e35c281ebfcf14b8a62071f76146eb0a5ede6fb43543a9c0ccddf3cf87fcdd0a96eea867595bb980dcea13e6283f16744631df895404434c7840f9b3d9c1069790a0459a0d35b7ae353519f5d437ded547f8d65f6c4916e988c842488e7a',
    };

    rawChainLockSigMessage = Buffer.from('00000020fd0ab0fc0fb0cbecb62cf7555aee6a8ce18564a9bbed8b22585d9f8563000000ee131c25019aaee0f1bdde2a5d6eb99ec0b4497e68776f18916951e8ddb6b922dd3be45f62f6011ed18800000103000500010000000000000000000000000000000000000000000000000000000000000000ffffffff05024c0f010bffffffff0200c817a8040000001976a91416b93a3b9168a20605cc3cda62f6135a3baa531a88ac00ac23fc060000001976a91416b93a3b9168a20605cc3cda62f6135a3baa531a88ac000000004602004c0f00003d8e273bf286d48ccba5a87b5adf332ed070a15e4e2d81eeb9ff685373be5656961e0b73ea855fdac9cc530782a7f0a22d25d1eaab4b2068efa647e9da0915d02f0f00005d3be29ed65ba0509d9e1936c593a0ce53f60875d16bea3fec1c0ef93d0000001770e35c281ebfcf14b8a62071f76146eb0a5ede6fb43543a9c0ccddf3cf87fcdd0a96eea867595bb980dcea13e6283f16744631df895404434c7840f9b3d9c1069790a0459a0d35b7ae353519f5d437ded547f8d65f6c4916e988c842488e7a', 'hex');

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

    const loggerMock = new LoggerMock(this.sinon);

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
    expect(coreZMQClientMock.subscribe).to.be.calledWith(ZMQClient.TOPICS.rawchainlocksig);
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
        expect(coreZMQClientMock.subscribe).to.be.calledWith(ZMQClient.TOPICS.rawchainlocksig);
        expect(coreZMQClientMock.subscribe).to.be.calledWith(ZMQClient.TOPICS.hashblock);
        expect(coreRpcClientMock.getBestChainLock).to.be.calledOnce();
        done();
      });

    setImmediate(() => {
      coreZMQClientMock.emit(
        ZMQClient.TOPICS.rawchainlocksig,
        rawChainLockSigMessage,
      );
    });
  });
});
