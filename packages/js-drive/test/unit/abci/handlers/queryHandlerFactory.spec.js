const cbor = require('cbor');

const GrpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');
const queryHandlerFactory = require('../../../../lib/abci/handlers/queryHandlerFactory');
const LoggerMock = require('../../../../lib/test/mock/LoggerMock');
const InvalidArgumentAbciError = require('../../../../lib/abci/errors/InvalidArgumentAbciError');

describe('queryHandlerFactory', () => {
  let queryHandler;
  let queryHandlerRouterMock;
  let sanitizeUrlMock;
  let request;
  let routeMock;
  let loggerMock;

  beforeEach(function beforeEach() {
    request = {
      path: '/identity',
      data: cbor.encode(Buffer.from('data')),
    };

    loggerMock = new LoggerMock(this.sinon);

    sanitizeUrlMock = this.sinon.stub();

    routeMock = {
      handler: this.sinon.stub(),
      params: 'params',
    };

    queryHandlerRouterMock = {
      find: this.sinon.stub().returns(routeMock),
    };

    queryHandler = queryHandlerFactory(
      queryHandlerRouterMock,
      sanitizeUrlMock,
      loggerMock,
    );
  });

  it('should throw InvalidArgumentAbciError if route was not found', async () => {
    const sanitizedUrl = 'sanitizedUrl';

    sanitizeUrlMock.returns(sanitizedUrl);
    queryHandlerRouterMock.find.returns(false);

    try {
      await queryHandler(request);

      expect.fail('should throw InvalidArgumentAbciError');
    } catch (e) {
      expect(e).to.be.instanceOf(InvalidArgumentAbciError);
      expect(e.getCode()).to.equal(GrpcErrorCodes.INVALID_ARGUMENT);

      expect(sanitizeUrlMock).to.be.calledOnceWith(request.path);
      expect(queryHandlerRouterMock.find).to.be.calledOnceWith('GET', sanitizedUrl);
      expect(routeMock.handler).to.be.not.called();
    }
  });

  it('should throw InvalidArgumentAbciError if fail to decode request data', async () => {
    const sanitizedUrl = 'sanitizedUrl';

    sanitizeUrlMock.returns(sanitizedUrl);

    request.data = Buffer.from('bb');

    try {
      await queryHandler(request);

      expect.fail('should throw InvalidArgumentAbciError');
    } catch (e) {
      expect(e).to.be.instanceOf(InvalidArgumentAbciError);
      expect(e.getCode()).to.equal(GrpcErrorCodes.INVALID_ARGUMENT);

      expect(sanitizeUrlMock).to.be.calledOnceWith(request.path);
      expect(queryHandlerRouterMock.find).to.be.calledOnceWith('GET', sanitizedUrl);
      expect(routeMock.handler).to.be.not.called();
    }
  });

  it('should throw InvalidArgumentAbciError on invalid request data', async () => {
    const sanitizedUrl = 'sanitizedUrl';

    sanitizeUrlMock.returns(sanitizedUrl);

    request.data = cbor.encode(null);

    try {
      await queryHandler(request);

      expect.fail('should throw InvalidArgumentAbciError');
    } catch (e) {
      expect(e).to.be.instanceOf(InvalidArgumentAbciError);
      expect(e.getCode()).to.equal(GrpcErrorCodes.INVALID_ARGUMENT);

      expect(sanitizeUrlMock).to.be.calledOnceWith(request.path);
      expect(queryHandlerRouterMock.find).to.be.calledOnceWith('GET', sanitizedUrl);
      expect(routeMock.handler).to.be.not.called();
    }
  });

  it('should call route handler without data');

  it('should call route handler and return response', async () => {
    const data = 'some data';
    const encodedData = cbor.decode(Buffer.from(request.data));
    const sanitizedUrl = 'sanitizedUrl';

    sanitizeUrlMock.returns(sanitizedUrl);
    routeMock.handler.resolves(data);
    queryHandlerRouterMock.find.returns(routeMock);

    const result = await queryHandler(request);

    expect(sanitizeUrlMock).to.be.calledOnceWith(request.path);
    expect(queryHandlerRouterMock.find).to.be.calledOnceWith('GET', sanitizedUrl);
    expect(routeMock.handler).to.be.calledOnceWith(routeMock.params, encodedData, request);
    expect(result).to.equal(data);
  });
});
