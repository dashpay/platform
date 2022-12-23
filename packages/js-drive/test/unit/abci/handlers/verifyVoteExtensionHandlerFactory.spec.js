const {
  tendermint: {
    abci: {
      ResponseVerifyVoteExtension,
    },
  },
} = require('@dashevo/abci/types');
const verifyVoteExtensionHandlerFactory = require('../../../../lib/abci/handlers/verifyVoteExtensionHandlerFactory');
const BlockExecutionContextMock = require('../../../../lib/test/mock/BlockExecutionContextMock');
const LoggerMock = require('../../../../lib/test/mock/LoggerMock');

describe('verifyVoteExtensionHandlerFactory', () => {
  let verifyVoteExtensionHandler;
  let proposalBlockExecutionContextMock;

  beforeEach(function beforeEach() {
    proposalBlockExecutionContextMock = new BlockExecutionContextMock(this.sinon);

    const loggerMock = new LoggerMock(this.sinon);
    proposalBlockExecutionContextMock.getConsensusLogger.returns(loggerMock);

    verifyVoteExtensionHandler = verifyVoteExtensionHandlerFactory(
      proposalBlockExecutionContextMock,
    );
  });

  it('should return ResponseVerifyVoteExtension', async () => {
    const result = await verifyVoteExtensionHandler();

    expect(result).to.be.an.instanceOf(ResponseVerifyVoteExtension);
    expect(result.status).to.equal(1);
  });
});
