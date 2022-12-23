const enrichErrorWithContextLoggerFactory = require('../../../../lib/abci/errors/enrichErrorWithContextLoggerFactory');
const BlockExecutionContextMock = require('../../../../lib/test/mock/BlockExecutionContextMock');
const LoggerMock = require('../../../../lib/test/mock/LoggerMock');

describe('enrichErrorWithContextLoggerFactory', () => {
  let blockExecutionContextMock;
  let enrichErrorWithContextLogger;
  let loggerMock;

  beforeEach(function beforeEach() {
    loggerMock = new LoggerMock(this.sinon);

    blockExecutionContextMock = new BlockExecutionContextMock(this.sinon);
    blockExecutionContextMock.contextLogger = loggerMock;

    enrichErrorWithContextLogger = enrichErrorWithContextLoggerFactory();
  });

  it('should add contextLogger from BlockExecutionContext to thrown error', async () => {
    const error = new Error();

    const method = () => {
      throw error;
    };

    const methodHandler = enrichErrorWithContextLogger(method);

    try {
      await methodHandler();

      expect.fail('should throw an error');
    } catch (e) {
      expect(e).to.equal(error);
      expect(e.contextLogger).to.equal(loggerMock);
    }
  });
});
