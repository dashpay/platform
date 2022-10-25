const GrpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');
const verifyChainLockFactory = require('../../../../../lib/abci/handlers/proposal/verifyChainLockFactory');

const LoggerMock = require('../../../../../lib/test/mock/LoggerMock');
const ChainlockVerificationFailedError = require('../../../../../lib/abci/errors/ChainlockVerificationFailedError');

describe('verifyChainLockFactory', () => {
  let verifyChainLock;
  let chainLockMock;
  let loggerMock;
  let coreRpcClientMock;

  beforeEach(function beforeEach() {
    chainLockMock = {
      toJSON: this.sinon.stub(),
    };
    chainLockMock.coreBlockHash = Buffer.alloc(0);
    chainLockMock.signature = Buffer.alloc(0);
    chainLockMock.coreBlockHeight = 42;

    loggerMock = new LoggerMock(this.sinon);

    coreRpcClientMock = {
      verifyChainLock: this.sinon.stub(),
    };
    coreRpcClientMock.verifyChainLock.resolves({ result: true });

    verifyChainLock = verifyChainLockFactory(
      coreRpcClientMock,
      loggerMock,
    );
  });

  it('should verify chain lock though Core', async () => {
    await verifyChainLock(chainLockMock);

    expect(coreRpcClientMock.verifyChainLock).to.be.calledOnceWithExactly(
      chainLockMock.coreBlockHash.toString('hex'),
      chainLockMock.signature.toString('hex'),
      chainLockMock.coreBlockHeight,
    );
  });

  it('should throw ChainlockVerificationFailedError if chainLock is not valid', async () => {
    coreRpcClientMock.verifyChainLock.returns(false);

    try {
      await verifyChainLock(chainLockMock);

      expect.fail('should throw InvalidArgumentAbciError');
    } catch (e) {
      expect(e).to.be.an.instanceof(ChainlockVerificationFailedError);
      expect(e.getCode()).to.equal(GrpcErrorCodes.INTERNAL);
      expect(e.message).to.equal('ChainLock verification failed: ChainLock is not valid');
    }
  });

  it('should throw ChainlockVerificationFailedError if Core returns parse error', async () => {
    const error = new Error('parse error');
    error.code = -32700;

    coreRpcClientMock.verifyChainLock.throws(error);

    try {
      await verifyChainLock(chainLockMock);

      expect.fail('should throw ChainlockVerificationFailedError');
    } catch (e) {
      expect(e).to.be.an.instanceof(ChainlockVerificationFailedError);
      expect(e.getCode()).to.equal(GrpcErrorCodes.INTERNAL);
      expect(e.message).to.equal(`ChainLock verification failed: ${error.message}`);

      expect(coreRpcClientMock.verifyChainLock).to.be.calledOnceWithExactly(
        chainLockMock.coreBlockHash.toString('hex'),
        chainLockMock.signature.toString('hex'),
        chainLockMock.coreBlockHeight,
      );
    }
  });

  it('should throw ChainlockVerificationFailedError if Core returns invalid signature format error', async () => {
    const error = new Error('invalid signature format');
    error.code = -8;

    coreRpcClientMock.verifyChainLock.throws(error);

    try {
      await verifyChainLock(chainLockMock);

      expect.fail('should throw ChainlockVerificationFailedError');
    } catch (e) {
      expect(e).to.be.an.instanceof(ChainlockVerificationFailedError);
      expect(e.getCode()).to.equal(GrpcErrorCodes.INTERNAL);
      expect(e.message).to.equal(`ChainLock verification failed: ${error.message}`);

      expect(coreRpcClientMock.verifyChainLock).to.be.calledOnceWithExactly(
        chainLockMock.coreBlockHash.toString('hex'),
        chainLockMock.signature.toString('hex'),
        chainLockMock.coreBlockHeight,
      );
    }
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
      chainLockMock.coreBlockHash.toString('hex'),
      chainLockMock.signature.toString('hex'),
      chainLockMock.coreBlockHeight,
    );
  });
});
