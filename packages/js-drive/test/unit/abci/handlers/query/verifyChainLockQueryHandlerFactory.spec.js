const {
  tendermint: {
    abci: {
      ResponseQuery,
    },
  },
} = require('@dashevo/abci/types');

const verifyChainLockQueryHandlerFactory = require('../../../../../lib/abci/handlers/query/verifyChainLockQueryHandlerFactory');

const InvalidArgumentAbciError = require('../../../../../lib/abci/errors/InvalidArgumentAbciError');

const AbciError = require('../../../../../lib/abci/errors/AbciError');

describe('verifyChainLockQueryHandlerFactory', () => {
  let simplifiedMasternodeListMock;
  let verifyChainLockQueryHandler;
  let params;
  let decodeChainLockMock;
  let encodedChainLock;
  let chainLockMock;
  let detectStandaloneRegtestModeMock;
  let loggerMock;

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

    loggerMock = {
      debug: this.sinon.stub(),
    };

    decodeChainLockMock = this.sinon.stub().returns(chainLockMock);
    detectStandaloneRegtestModeMock = this.sinon.stub().returns(false);

    encodedChainLock = Buffer.alloc(0);
  });

  it('should validate a valid chainLock', async () => {
    chainLockMock.verify.returns(true);

    verifyChainLockQueryHandler = verifyChainLockQueryHandlerFactory(
      simplifiedMasternodeListMock,
      decodeChainLockMock,
      detectStandaloneRegtestModeMock,
      loggerMock,
    );

    const result = await verifyChainLockQueryHandler(params, encodedChainLock);

    expect(result).to.be.an.instanceof(ResponseQuery);
    expect(result.code).to.equal(0);

    expect(decodeChainLockMock).to.be.calledOnceWithExactly(encodedChainLock);
  });

  it('should throw InvalidArgumentAbciError if chainLock is not valid', async () => {
    chainLockMock.verify.returns(false);

    verifyChainLockQueryHandler = verifyChainLockQueryHandlerFactory(
      simplifiedMasternodeListMock,
      decodeChainLockMock,
      detectStandaloneRegtestModeMock,
      loggerMock,
    );

    try {
      await verifyChainLockQueryHandler(params, encodedChainLock);

      expect.fail('should throw InvalidArgumentAbciError');
    } catch (e) {
      expect(e).to.be.an.instanceof(InvalidArgumentAbciError);
      expect(e.getCode()).to.equal(AbciError.CODES.INVALID_ARGUMENT);
      expect(e.message).to.equal('Signature invalid for chainLock');
    }
  });

  it('should not validate chainLock in standalone regtest mode', async () => {
    detectStandaloneRegtestModeMock.returns(true);
    chainLockMock.verify.returns(false);

    verifyChainLockQueryHandler = verifyChainLockQueryHandlerFactory(
      simplifiedMasternodeListMock,
      decodeChainLockMock,
      detectStandaloneRegtestModeMock,
      loggerMock,
    );

    const result = await verifyChainLockQueryHandler(params, { chainLock: encodedChainLock });

    expect(result).to.be.an.instanceof(ResponseQuery);
    expect(result.code).to.equal(0);
  });

  it('should throw an error if SML store is missing', async () => {
    simplifiedMasternodeListMock.getStore.returns(undefined);

    verifyChainLockQueryHandler = verifyChainLockQueryHandlerFactory(
      simplifiedMasternodeListMock,
      decodeChainLockMock,
      detectStandaloneRegtestModeMock,
      loggerMock,
    );

    try {
      await verifyChainLockQueryHandler(params, { chainLock: encodedChainLock });

      expect.fail('error was not thrown');
    } catch (e) {
      expect(e.message).to.equal('SML Store is not defined for verify chain lock handler');
    }
  });
});
