const EventEmitter = require('events');
const waitForChainlockedHeightFactory = require('../../../lib/core/waitForChainLockedHeightFactory');
const MissingChainlockError = require('../../../lib/core/errors/MissingChainLockError');
const LatestCoreChainLock = require('../../../lib/core/LatestCoreChainLock');

describe('waitForChainLockedHeightFactory', () => {
  let waitForChainlockedHeight;
  let latestCoreChainLockMock;
  let chainLock;
  let coreHeight;

  beforeEach(function beforeEach() {
    coreHeight = 84202;

    chainLock = {
      height: coreHeight,
      signature: '0a43f1c3e5b3e8dbd670bca8d437dc25572f72d8e1e9be673e9ebbb606570307c3e5f5d073f7beb209dd7e0b8f96c751060ab3a7fb69a71d5ccab697b8cfa5a91038a6fecf76b7a827d75d17f01496302942aa5e2c7f4a48246efc8d3941bf6c',
    };

    latestCoreChainLockMock = new EventEmitter();
    latestCoreChainLockMock.getChainLock = this.sinon.stub().returns(chainLock);

    waitForChainlockedHeight = waitForChainlockedHeightFactory(
      latestCoreChainLockMock,
    );
  });

  it('should throw MissingChainlockError if chainlock is empty', async () => {
    latestCoreChainLockMock.getChainLock.returns(null);

    try {
      await waitForChainlockedHeight(coreHeight);

      expect.fail();
    } catch (e) {
      expect(e).to.be.an.instanceOf(MissingChainlockError);
    }
  });

  it('should resolve promise if existing chainlock on the same height or higher', async () => {
    latestCoreChainLockMock.getChainLock.returns(chainLock);

    await waitForChainlockedHeight(coreHeight);
  });

  it('should resolve when chainLock height to be equal or higher', (done) => {
    coreHeight = chainLock.height + 1;

    waitForChainlockedHeight(coreHeight)
      .then(() => {
        expect(latestCoreChainLockMock.getChainLock).to.have.been.calledOnce();

        done();
      });

    setImmediate(() => {
      latestCoreChainLockMock.emit(LatestCoreChainLock.EVENTS.update, {
        ...chainLock,
        height: chainLock.height + 1,
      });
    });
  });
});
