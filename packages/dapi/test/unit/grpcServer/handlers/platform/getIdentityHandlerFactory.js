const {
  server: {
    error: {
      InvalidArgumentGrpcError,
    },
  },
} = require('@dashevo/grpc-common');

const {
  GetIdentityResponse,
} = require('@dashevo/dapi-grpc');

const getIdentityHandlerFactory = require('../../../../../lib/grpcServer/handlers/platform/getIdentityHandlerFactory');

const GrpcCallMock = require('../../../../../lib/test/mock/GrpcCallMock');

describe('getIdentityHandlerFactory', () => {
  let call;
  let rpcClientMock;
  let id;
  let handleResponseMock;
  let getIdentityHandler;
  let response;
  let rpcResponse;
  let hexId;

  beforeEach(function beforeEach() {
    id = '5poV8Vdi27VksX2RAzAgXmjAh14y87JN2zLvyAwmepRK';
    call = new GrpcCallMock(this.sinon, {
      getId: this.sinon.stub().returns(id),
    });

    handleResponseMock = this.sinon.stub();

    const code = 0;

    const log = JSON.stringify({
      error: {
        message: 'some message',
        data: {
          error: 'some data',
        },
      },
    });

    const value = Buffer.from('value');

    response = {
      value,
      log,
      code,
    };

    rpcResponse = {
      id: '',
      jsonrpc: '2.0',
      error: '',
      result: {
        response,
      },
    };

    hexId = Buffer.from(id).toString('hex');

    rpcClientMock = {
      request: this.sinon.stub().resolves(rpcResponse),
      getId: this.sinon.stub(),
    };

    getIdentityHandler = getIdentityHandlerFactory(rpcClientMock, handleResponseMock);
  });

  it('should return valid result', async () => {
    const result = await getIdentityHandler(call);

    expect(result).to.be.an.instanceOf(GetIdentityResponse);

    expect(rpcClientMock.request).to.be.calledOnceWith('abci_query', { path: '/identity', data: hexId });
    expect(handleResponseMock).to.be.calledOnceWith(response);
  });

  it('should throw an InvalidArgumentGrpcError if id is not specified', async () => {
    call.request.getId.returns(null);

    try {
      await getIdentityHandler(call);

      expect.fail('should throw an error');
    } catch (e) {
      expect(e).to.be.instanceOf(InvalidArgumentGrpcError);
      expect(e.getMessage()).to.equal('id is not specified');
      expect(rpcClientMock.request).to.not.be.called();
      expect(handleResponseMock).to.not.be.called();
    }
  });

  it('should throw an error when handleResponse throws an error', async () => {
    const error = new Error();
    handleResponseMock.throws(error);

    try {
      await getIdentityHandler(call);

      expect.fail('should throw an error');
    } catch (e) {
      expect(e).to.equal(error);
      expect(handleResponseMock).to.be.calledOnceWith(response);
    }
  });
});
