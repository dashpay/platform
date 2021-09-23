const enrichErrorWithConsensusLoggerFactory = require('../../../../lib/abci/errors/enrichErrorWithConsensusLoggerFactory');
const BlockExecutionContextMock = require('../../../../lib/test/mock/BlockExecutionContextMock');
const LoggerMock = require('../../../../lib/test/mock/LoggerMock');

describe('enrichErrorWithConsensusLoggerFactory', () => {
  let blockExecutionContextMock;
  let enrichErrorWithConsensusLogger;
  let loggerMock;

  beforeEach(function beforeEach() {
    loggerMock = new LoggerMock(this.sinon);

    blockExecutionContextMock = new BlockExecutionContextMock(this.sinon);
    blockExecutionContextMock.consensusLogger = loggerMock;

    enrichErrorWithConsensusLogger = enrichErrorWithConsensusLoggerFactory(
      blockExecutionContextMock,
    );
  });

  it('should add consensusLogger from BlockExecutionContext to thrown error', async () => {
    const error = new Error();

    const method = () => {
      throw error;
    };

    const methodHandler = enrichErrorWithConsensusLogger(method);

    try {
      await methodHandler();

      expect.fail('should throw an error');
    } catch (e) {
      expect(e).to.equal(error);
      expect(e.consensusLogger).to.equal(loggerMock);
    }
  });
});
