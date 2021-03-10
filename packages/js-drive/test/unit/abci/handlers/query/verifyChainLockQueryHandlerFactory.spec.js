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
const LoggerMock = require('../../../../../lib/test/mock/LoggerMock');

describe('verifyChainLockQueryHandlerFactory', () => {
  let simplifiedMasternodeListMock;
  let verifyChainLockQueryHandler;
  let params;
  let decodeChainLockMock;
  let encodedChainLock;
  let chainLockMock;
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

    loggerMock = new LoggerMock(this.sinon);

    decodeChainLockMock = this.sinon.stub().returns(chainLockMock);

    encodedChainLock = Buffer.alloc(0);

    verifyChainLockQueryHandler = verifyChainLockQueryHandlerFactory(
      simplifiedMasternodeListMock,
      decodeChainLockMock,
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
    chainLockMock.verify.returns(false);

    try {
      await verifyChainLockQueryHandler(params, encodedChainLock);

      expect.fail('should throw InvalidArgumentAbciError');
    } catch (e) {
      expect(e).to.be.an.instanceof(InvalidArgumentAbciError);
      expect(e.getCode()).to.equal(AbciError.CODES.INVALID_ARGUMENT);
      expect(e.message).to.equal('Signature invalid for chainLock');
    }
  });

  it('should throw an error if SML store is missing', async () => {
    simplifiedMasternodeListMock.getStore.returns(undefined);

    try {
      await verifyChainLockQueryHandler(params, { chainLock: encodedChainLock });

      expect.fail('error was not thrown');
    } catch (e) {
      expect(e.message).to.equal('SML Store is not defined for verify chain lock handler');
    }
  });
});
