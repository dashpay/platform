const {
  tendermint: {
    abci: {
      ResponseQuery,
    },
  },
} = require('@dashevo/abci/types');

const GrpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');
const verifyChainLockQueryHandlerFactory = require('../../../../../lib/abci/handlers/query/verifyChainLockQueryHandlerFactory');

const LoggerMock = require('../../../../../lib/test/mock/LoggerMock');
const BlockExecutionContextMock = require('../../../../../lib/test/mock/BlockExecutionContextMock');
const InvalidArgumentAbciError = require('../../../../../lib/abci/errors/InvalidArgumentAbciError');

describe('verifyChainLockQueryHandlerFactory', () => {
  let simplifiedMasternodeListMock;
  let verifyChainLockQueryHandler;
  let params;
  let decodeChainLockMock;
  let encodedChainLock;
  let chainLockMock;
  let loggerMock;
  let getLatestFeatureFlagMock;
  let blockExecutionContextMock;
  let coreRpcClientMock;

  beforeEach(function beforeEach() {
    params = {};

    simplifiedMasternodeListMock = {
      getStore: this.sinon.stub(),
    };

    simplifiedMasternodeListMock.getStore.returns({
      tipHeight: 42,
    });

    chainLockMock = {
      verify: this.sinon.stub(),
      toJSON: this.sinon.stub(),
    };
    chainLockMock.blockHash = Buffer.alloc(0);
    chainLockMock.signature = Buffer.alloc(0);
    chainLockMock.height = 42;

    loggerMock = new LoggerMock(this.sinon);

    decodeChainLockMock = this.sinon.stub().returns(chainLockMock);

    encodedChainLock = Buffer.alloc(0);

    getLatestFeatureFlagMock = this.sinon.stub();
    getLatestFeatureFlagMock.resolves(null);

    blockExecutionContextMock = new BlockExecutionContextMock(this.sinon);
    blockExecutionContextMock.getHeader.returns({
      height: 42,
      coreChainLockedHeight: 43,
    });

    coreRpcClientMock = {
      verifyChainLock: this.sinon.stub(),
    };
    coreRpcClientMock.verifyChainLock.resolves({ result: true });

    verifyChainLockQueryHandler = verifyChainLockQueryHandlerFactory(
      simplifiedMasternodeListMock,
      decodeChainLockMock,
      getLatestFeatureFlagMock,
      blockExecutionContextMock,
      coreRpcClientMock,
      loggerMock,
    );
  });

  it('should validate a valid chainLock', async () => {
    chainLockMock.verify.returns(true);

    const result = await verifyChainLockQueryHandler(params, encodedChainLock);

    expect(result).to.be.an.instanceof(ResponseQuery);
    expect(result.code).to.equal(0);

    expect(decodeChainLockMock).to.be.calledOnceWithExactly(encodedChainLock);
  });

  it('should throw InvalidArgumentAbciError if chainLock is not valid', async () => {
    coreRpcClientMock.verifyChainLock.returns(false);

    try {
      await verifyChainLockQueryHandler(params, encodedChainLock);

      expect.fail('should throw InvalidArgumentAbciError');
    } catch (e) {
      expect(e).to.be.an.instanceof(InvalidArgumentAbciError);
      expect(e.getCode()).to.equal(GrpcErrorCodes.INVALID_ARGUMENT);
      expect(e.message).to.equal('ChainLock verification failed');
    }
  });

  it('should verify chain lock though Core', async () => {
    const result = await verifyChainLockQueryHandler(params, encodedChainLock);

    expect(result).to.be.an.instanceof(ResponseQuery);
    expect(result.code).to.equal(0);

    expect(decodeChainLockMock).to.be.calledOnceWithExactly(encodedChainLock);
    expect(coreRpcClientMock.verifyChainLock).to.be.calledOnceWithExactly(
      chainLockMock.blockHash.toString('hex'),
      chainLockMock.signature.toString('hex'),
      chainLockMock.height,
    );
  });

  it('should return false if Core returns parse error', async () => {
    const error = new Error();
    error.code = -32700;

    coreRpcClientMock.verifyChainLock.throws(error);

    const result = await verifyChainLockQueryHandler(params, encodedChainLock);

    expect(result).to.be.equal(false);

    expect(decodeChainLockMock).to.be.calledOnceWithExactly(encodedChainLock);
    expect(coreRpcClientMock.verifyChainLock).to.be.calledOnceWithExactly(
      chainLockMock.blockHash.toString('hex'),
      chainLockMock.signature.toString('hex'),
      chainLockMock.height,
    );
  });

  it('should return false if Core returns invalid signature format error', async () => {
    const error = new Error();
    error.code = -8;

    coreRpcClientMock.verifyChainLock.throws(error);

    const result = await verifyChainLockQueryHandler(params, encodedChainLock);

    expect(result).to.be.equal(false);

    expect(decodeChainLockMock).to.be.calledOnceWithExactly(encodedChainLock);
    expect(coreRpcClientMock.verifyChainLock).to.be.calledOnceWithExactly(
      chainLockMock.blockHash.toString('hex'),
      chainLockMock.signature.toString('hex'),
      chainLockMock.height,
    );
  });

  it('should throw an error if Core throws error', async () => {
    const error = new Error();

    coreRpcClientMock.verifyChainLock.throws(error);

    try {
      await verifyChainLockQueryHandler(params, encodedChainLock);

      expect.fail('error was not thrown');
    } catch (e) {
      expect(e).to.deep.equal(error);
    }

    expect(decodeChainLockMock).to.be.calledOnceWithExactly(encodedChainLock);
    expect(coreRpcClientMock.verifyChainLock).to.be.calledOnceWithExactly(
      chainLockMock.blockHash.toString('hex'),
      chainLockMock.signature.toString('hex'),
      chainLockMock.height,
    );
  });
});
