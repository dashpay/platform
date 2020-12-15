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

  beforeEach(function beforeEach() {
    params = {};

    simplifiedMasternodeListMock = {
      getStore: this.sinon.stub(),
    };

    chainLockMock = {
      verify: this.sinon.stub(),
      toJSON: this.sinon.stub(),
    };

    decodeChainLockMock = this.sinon.stub().returns(chainLockMock);

    encodedChainLock = Buffer.alloc(0);
  });

  it('should validate a valid chainlock', async () => {
    chainLockMock.verify.returns(true);

    verifyChainLockQueryHandler = verifyChainLockQueryHandlerFactory(
      simplifiedMasternodeListMock, decodeChainLockMock,
    );

    const result = await verifyChainLockQueryHandler(params, { chainLock: encodedChainLock });

    expect(result).to.be.an.instanceof(ResponseQuery);
    expect(result.code).to.equal(0);

    expect(decodeChainLockMock).to.be.calledOnceWithExactly(encodedChainLock);
  });

  it('should throw InvalidArgumentAbciError if chainlock is not valid', async () => {
    chainLockMock.verify.returns(false);

    verifyChainLockQueryHandler = verifyChainLockQueryHandlerFactory(
      simplifiedMasternodeListMock, decodeChainLockMock,
    );

    try {
      await verifyChainLockQueryHandler(params, { chainLock: encodedChainLock });

      expect.fail('should throw InvalidArgumentAbciError');
    } catch (e) {
      expect(e).to.be.an.instanceof(InvalidArgumentAbciError);
      expect(e.getCode()).to.equal(AbciError.CODES.INVALID_ARGUMENT);
      expect(e.message).to.equal('Signature invalid for chainlock');
    }
  });
});
