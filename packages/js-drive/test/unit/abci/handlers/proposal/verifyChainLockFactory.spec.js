const verifyChainLockFactory = require('../../../../../lib/abci/handlers/proposal/verifyChainLockFactory');

const LoggerMock = require('../../../../../lib/test/mock/LoggerMock');
const BlockExecutionContextMock = require('../../../../../lib/test/mock/BlockExecutionContextMock');

describe('verifyChainLockFactory', () => {
  let verifyChainLock;
  let chainLockMock;
  let loggerMock;
  let coreRpcClientMock;
  let latestBlockExecutionContextMock;

  beforeEach(function beforeEach() {
    latestBlockExecutionContextMock = new BlockExecutionContextMock(this.sinon);
    latestBlockExecutionContextMock.getCoreChainLockedHeight.returns(41);

    chainLockMock = {
      coreBlockHash: Buffer.alloc(1, 1).toString(),
      signature: Buffer.alloc(1, 2).toString(),
      coreBlockHeight: 42,
    };

    loggerMock = new LoggerMock(this.sinon);

    coreRpcClientMock = {
      verifyChainLock: this.sinon.stub(),
    };
    coreRpcClientMock.verifyChainLock.resolves({ result: true });

    verifyChainLock = verifyChainLockFactory(
      coreRpcClientMock,
      latestBlockExecutionContextMock,
      loggerMock,
    );
  });

  it('should verify chain lock though Core', async () => {
    const result = await verifyChainLock(chainLockMock);

    expect(result).to.be.true();
    expect(coreRpcClientMock.verifyChainLock).to.be.calledOnceWithExactly(
      Buffer.from(chainLockMock.coreBlockHash).toString('hex'),
      Buffer.from(chainLockMock.signature).toString('hex'),
      chainLockMock.coreBlockHeight,
    );

    expect(latestBlockExecutionContextMock.getCoreChainLockedHeight).to.be.calledOnce();
  });

  it('should return false if chainLock is not valid', async () => {
    coreRpcClientMock.verifyChainLock.resolves({ result: false });

    const result = await verifyChainLock(chainLockMock);

    expect(result).to.be.false();
  });

  it('should return false if Core returns parse error', async () => {
    const error = new Error('parse error');
    error.code = -32700;

    coreRpcClientMock.verifyChainLock.throws(error);

    const result = await verifyChainLock(chainLockMock);

    expect(result).to.be.false();
    expect(coreRpcClientMock.verifyChainLock).to.be.calledOnceWithExactly(
      Buffer.from(chainLockMock.coreBlockHash).toString('hex'),
      Buffer.from(chainLockMock.signature).toString('hex'),
      chainLockMock.coreBlockHeight,
    );
  });

  it('should return false if Core returns invalid signature format error', async () => {
    const error = new Error('invalid signature format');
    error.code = -8;

    coreRpcClientMock.verifyChainLock.throws(error);

    const result = await verifyChainLock(chainLockMock);

    expect(result).to.be.false();
    expect(coreRpcClientMock.verifyChainLock).to.be.calledOnceWithExactly(
      Buffer.from(chainLockMock.coreBlockHash).toString('hex'),
      Buffer.from(chainLockMock.signature).toString('hex'),
      chainLockMock.coreBlockHeight,
    );
  });

  it('should throw an error if Core throws error', async () => {
    const error = new Error();

    coreRpcClientMock.verifyChainLock.throws(error);

    try {
      await verifyChainLock(chainLockMock);

      expect.fail('error was not thrown');
    } catch (e) {
      expect(e).to.deep.equal(error);
    }

    expect(coreRpcClientMock.verifyChainLock).to.be.calledOnceWithExactly(
      Buffer.from(chainLockMock.coreBlockHash).toString('hex'),
      Buffer.from(chainLockMock.signature).toString('hex'),
      chainLockMock.coreBlockHeight,
    );
  });

  it('should return false if coreBlockHeight >= lastCoreChainLockedHeight', async () => {
    latestBlockExecutionContextMock.getCoreChainLockedHeight.returns(42);
    const result = await verifyChainLock(chainLockMock);

    expect(result).to.be.false();
    expect(coreRpcClientMock.verifyChainLock).to.not.be.called();
  });
});
