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

    wrapInErrorHandler = wrapInErrorHandlerFactory(loggerMock);
    methodMock = this.sinon.stub();

    handler = wrapInErrorHandler(
      methodMock, { throwNonABCIErrors: false },
    );
  });

  it('should respond with internal error code if any Error is thrown in handler', async () => {
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
      tags: [],
    });
  });

  it('should respond with internal error code if an InternalAbciError is thrown in handler', async () => {
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
      tags: [],
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
      tags: [],
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
});
