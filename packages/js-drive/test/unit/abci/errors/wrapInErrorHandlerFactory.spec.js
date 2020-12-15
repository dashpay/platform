const wrapInErrorHandlerFactory = require('../../../../lib/abci/errors/wrapInErrorHandlerFactory');

const InternalAbciError = require('../../../../lib/abci/errors/InternalAbciError');
const InvalidArgumentAbciError = require('../../../../lib/abci/errors/InvalidArgumentAbciError');

describe('wrapInErrorHandlerFactory', () => {
  let loggerMock;
  let methodMock;
  let request;
  let handler;
  let wrapInErrorHandler;

  beforeEach(function beforeEach() {
    request = {
      tx: Buffer.alloc(0),
    };

    loggerMock = {
      error: this.sinon.stub(),
    };

    wrapInErrorHandler = wrapInErrorHandlerFactory(loggerMock, true);
    methodMock = this.sinon.stub();

    handler = wrapInErrorHandler(
      methodMock,
    );
  });

  it('should throw an internal error if any Error is thrown in handler', async () => {
    const error = new Error('Custom error');

    methodMock.throws(error);

    try {
      await handler(request);

      expect.fail('Internal error must be thrown');
    } catch (e) {
      expect(e).to.equal(error);
    }
  });

  it('should throw en internal error if an InternalAbciError is thrown in handler', async () => {
    const originError = new Error();
    const data = { sample: 'data' };
    const error = new InternalAbciError(originError, data);

    methodMock.throws(error);

    try {
      await handler(request);

      expect.fail('Internal error must be thrown');
    } catch (e) {
      expect(e).to.equal(originError);
    }
  });

  it('should respond with internal error code if any Error is thrown in handler and respondWithInternalError enabled', async () => {
    handler = wrapInErrorHandler(
      methodMock, { respondWithInternalError: true },
    );

    const error = new Error('Custom error');

    methodMock.throws(error);

    const response = await handler(request);

    expect(response).to.deep.equal({
      code: 1,
      log: JSON.stringify({
        error: {
          message: 'Internal error',
        },
      }),
      events: [],
    });
  });

  it('should respond with internal error code if an InternalAbciError is thrown in handler and respondWithInternalError enabled', async () => {
    handler = wrapInErrorHandler(
      methodMock, { respondWithInternalError: true },
    );

    const data = { sample: 'data' };
    const error = new InternalAbciError(new Error(), data);

    methodMock.throws(error);

    const response = await handler(request);

    expect(response).to.deep.equal({
      code: error.getCode(),
      log: JSON.stringify({
        error: {
          message: error.getMessage(),
          data: error.getData(),
        },
      }),
      events: [],
    });
  });

  it('should respond with invalid argument error if it is thrown in handler', async () => {
    const data = { sample: 'data' };
    const error = new InvalidArgumentAbciError('test', data);

    methodMock.throws(error);

    const response = await handler(request);

    expect(response).to.deep.equal({
      code: error.getCode(),
      log: JSON.stringify({
        error: {
          message: error.getMessage(),
          data: error.getData(),
        },
      }),
      events: [],
    });
  });

  it('should rethrow internal errors in case `rethrowInternalErrors` options is set', async () => {
    const unknownError = new Error('Some internal error indicating a bug');

    methodMock.throws(unknownError);

    handler = wrapInErrorHandler(
      methodMock, { throwNonABCIErrors: true },
    );

    try {
      await handler(request);
      expect.fail('Error was not re-thrown');
    } catch (e) {
      expect(e).to.equal(unknownError);
    }
  });

  it('should respond with verbose error containing message and stack in debug mode', async () => {
    wrapInErrorHandler = wrapInErrorHandlerFactory(loggerMock, false);

    const error = new Error('Custom error');

    methodMock.throws(error);

    handler = wrapInErrorHandler(
      methodMock, { respondWithInternalError: true },
    );

    methodMock.throws(error);

    const response = await handler(request);

    const [, errorPath] = error.stack.toString().split(/\r\n|\n/);

    expect(response).to.deep.equal({
      code: 1,
      log: JSON.stringify({
        error: {
          message: `${error.message} ${errorPath.trim()}`,
          data: {
            stack: error.stack,
            data: undefined,
          },
        },
      }),
      events: [],
    });
  });
});
