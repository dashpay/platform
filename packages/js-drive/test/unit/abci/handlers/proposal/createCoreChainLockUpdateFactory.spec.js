const {
  tendermint: {
    types: {
      CoreChainLock,
    },
  },
} = require('@dashevo/abci/types');
const createCoreChainLockUpdateFactory = require('../../../../../lib/abci/handlers/proposal/createCoreChainLockUpdateFactory');
const BlockExecutionContextMock = require('../../../../../lib/test/mock/BlockExecutionContextMock');
const LoggerMock = require('../../../../../lib/test/mock/LoggerMock');

describe('createCoreChainLockUpdateFactory', () => {
  let createCoreChainLockUpdate;
  let blockExecutionContextMock;
  let latestCoreChainLockMock;
  let chainLockMock;
  let coreChainLockedHeight;
  let loggerMock;
  let round;
  let proposalBlockExecutionContextCollectionMock;

  beforeEach(function beforeEach() {
    round = 0;
    loggerMock = new LoggerMock(this.sinon);

    chainLockMock = {
      height: 1,
      blockHash: Buffer.alloc(0),
      signature: Buffer.alloc(0),
    };

    coreChainLockedHeight = 2;

    blockExecutionContextMock = new BlockExecutionContextMock(this.sinon);

    blockExecutionContextMock.hasDataContract.returns(true);
    blockExecutionContextMock.getCoreChainLockedHeight.returns(coreChainLockedHeight);

    latestCoreChainLockMock = {
      getChainLock: this.sinon.stub().returns(chainLockMock),
    };

    proposalBlockExecutionContextCollectionMock = {
      get: this.sinon.stub().returns(blockExecutionContextMock),
    };

    createCoreChainLockUpdate = createCoreChainLockUpdateFactory(
      proposalBlockExecutionContextCollectionMock,
      latestCoreChainLockMock,
    );
  });

  it('should return nextCoreChainLockUpdate if latestCoreChainLock above header height', async () => {
    chainLockMock.height = 3;

    const response = await createCoreChainLockUpdate(round, loggerMock);

    expect(proposalBlockExecutionContextCollectionMock.get).to.have.been.calledOnceWithExactly(
      round,
    );
    expect(latestCoreChainLockMock.getChainLock).to.have.been.calledOnceWithExactly();

    const expectedCoreChainLock = new CoreChainLock({
      coreBlockHeight: chainLockMock.height,
      coreBlockHash: chainLockMock.blockHash,
      signature: chainLockMock.signature,
    });

    expect(response).to.deep.equal(expectedCoreChainLock);
  });

  it('should return undefined', async () => {
    chainLockMock.height = 1;

    const response = await createCoreChainLockUpdate(round, loggerMock);

    expect(proposalBlockExecutionContextCollectionMock.get).to.have.been.calledOnceWithExactly(
      round,
    );
    expect(latestCoreChainLockMock.getChainLock).to.have.been.calledOnceWithExactly();

    expect(response).to.be.undefined();
  });
});
