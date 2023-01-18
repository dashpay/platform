const {
  tendermint: {
    abci: {
      ResponseVerifyVoteExtension,
    },
    types: {
      VoteExtensionType,
    },
  },
} = require('@dashevo/abci/types');
const verifyVoteExtensionHandlerFactory = require('../../../../lib/abci/handlers/verifyVoteExtensionHandlerFactory');
const BlockExecutionContextMock = require('../../../../lib/test/mock/BlockExecutionContextMock');
const LoggerMock = require('../../../../lib/test/mock/LoggerMock');

describe('verifyVoteExtensionHandlerFactory', () => {
  let verifyVoteExtensionHandler;
  let proposalBlockExecutionContextMock;
  let unsignedWithdrawalTransactionsMapMock;

  beforeEach(function beforeEach() {
    proposalBlockExecutionContextMock = new BlockExecutionContextMock(this.sinon);

    const loggerMock = new LoggerMock(this.sinon);
    proposalBlockExecutionContextMock.getContextLogger.returns(loggerMock);

    unsignedWithdrawalTransactionsMapMock = {};
    proposalBlockExecutionContextMock.getWithdrawalTransactionsMap.returns(
      unsignedWithdrawalTransactionsMapMock,
    );

    verifyVoteExtensionHandler = verifyVoteExtensionHandlerFactory(
      proposalBlockExecutionContextMock,
    );
  });

  it('should return ResponseVerifyVoteExtension with REJECT status if vote extensions length not match', async () => {
    const voteExtensions = [
      { type: VoteExtensionType.THRESHOLD_RECOVER, extension: Buffer.alloc(32, 1) },
      { type: VoteExtensionType.THRESHOLD_RECOVER, extension: Buffer.alloc(32, 2) },
      { type: VoteExtensionType.THRESHOLD_RECOVER, extension: Buffer.alloc(32, 3) },
    ];

    const unsignedWithdrawalTransactionsMap = {
      [Buffer.alloc(32, 1).toString('hex')]: undefined,
      [Buffer.alloc(32, 2).toString('hex')]: undefined,
    };

    proposalBlockExecutionContextMock.getWithdrawalTransactionsMap.returns(
      unsignedWithdrawalTransactionsMap,
    );

    const result = await verifyVoteExtensionHandler({ voteExtensions });

    expect(result).to.be.an.instanceOf(ResponseVerifyVoteExtension);
    expect(result.status).to.equal(2);
  });

  it('should return ResponseVerifyVoteExtension with REJECT status if vote extension is missing', async () => {
    const voteExtensions = [
      { type: VoteExtensionType.THRESHOLD_RECOVER, extension: Buffer.alloc(32, 1) },
    ];

    const unsignedWithdrawalTransactionsMap = {
      [Buffer.alloc(32, 1).toString('hex')]: undefined,
      [Buffer.alloc(32, 2).toString('hex')]: undefined,
    };

    proposalBlockExecutionContextMock.getWithdrawalTransactionsMap.returns(
      unsignedWithdrawalTransactionsMap,
    );

    const result = await verifyVoteExtensionHandler({ voteExtensions });

    expect(result).to.be.an.instanceOf(ResponseVerifyVoteExtension);
    expect(result.status).to.equal(2);
  });

  it('should return ACCEPT if everything is fine', async () => {
    const voteExtensions = [
      { type: VoteExtensionType.THRESHOLD_RECOVER, extension: Buffer.alloc(32, 1) },
      { type: VoteExtensionType.THRESHOLD_RECOVER, extension: Buffer.alloc(32, 2) },
    ];

    const unsignedWithdrawalTransactionsMap = {
      [Buffer.alloc(32, 1).toString('hex')]: undefined,
      [Buffer.alloc(32, 2).toString('hex')]: undefined,
    };

    proposalBlockExecutionContextMock.getWithdrawalTransactionsMap.returns(
      unsignedWithdrawalTransactionsMap,
    );

    const result = await verifyVoteExtensionHandler({ voteExtensions });

    expect(result).to.be.an.instanceOf(ResponseVerifyVoteExtension);
    expect(result.status).to.equal(1);
  });
});
