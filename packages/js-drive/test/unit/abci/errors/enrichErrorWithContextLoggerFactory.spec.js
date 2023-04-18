const { AsyncLocalStorage } = require('node:async_hooks');
const enrichErrorWithContextLoggerFactory = require('../../../../lib/abci/errors/enrichErrorWithContextLoggerFactory');
const LoggerMock = require('../../../../lib/test/mock/LoggerMock');

describe('enrichErrorWithContextLoggerFactory', () => {
  let enrichErrorWithContextLogger;
  let loggerMock;
  let asyncLocalStorage;

  beforeEach(function beforeEach() {
    loggerMock = new LoggerMock(this.sinon);

    asyncLocalStorage = new AsyncLocalStorage();

    enrichErrorWithContextLogger = enrichErrorWithContextLoggerFactory(asyncLocalStorage);
  });

  it('should add contextLogger from BlockExecutionContext to thrown error', async () => {
    const error = new Error('my error');

    const method = async () => {
      asyncLocalStorage.getStore().set('logger', loggerMock);

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
