import DAPIAddress from '../../../../lib/dapiAddressProvider/DAPIAddress.js';
import WrongHttpCodeError from '../../../../lib/transport/JsonRpcTransport/errors/WrongHttpCodeError.js';
import createJsonTransportError from '../../../../lib/transport/JsonRpcTransport/createJsonTransportError.js';
import ServerError from '../../../../lib/transport/errors/response/ServerError.js';
import JsonRpcError from '../../../../lib/transport/JsonRpcTransport/errors/JsonRpcError.js';
import ResponseError from '../../../../lib/transport/errors/response/ResponseError.js';
import RetriableResponseError from '../../../../lib/transport/errors/response/RetriableResponseError.js';

describe('createJsonTransportError', () => {
  let dapiAddress;
  let requestInfo;

  beforeEach(() => {
    dapiAddress = new DAPIAddress('127.0.0.1');
  });

  it('should return ServerError', () => {
    requestInfo = {
      host: '127.0.0.1',
      port: 80,
      method: 'someMethod',
      params: {},
      options: {},
    };
    const statusCode = 500;
    const statusMessage = 'status message';

    const error = new WrongHttpCodeError(requestInfo, statusCode, statusMessage);

    const jsonTransportError = createJsonTransportError(error, dapiAddress);

    expect(jsonTransportError).to.be.an.instanceOf(ServerError);
    expect(jsonTransportError.getDAPIAddress()).to.deep.equal(dapiAddress);
    expect(jsonTransportError.getCode()).to.equal(statusCode);
    expect(jsonTransportError.getData()).to.deep.equal({});
    expect(jsonTransportError.message).to.equal(error.message);
  });

  it('should return ResponseError on JsonRpcError', () => {
    const jsonRpcError = {
      code: -32000,
      message: 'error message',
      data: {
        info: 'some data',
      },
    };

    const error = new JsonRpcError(requestInfo, jsonRpcError);

    const jsonTransportError = createJsonTransportError(error, dapiAddress);

    expect(jsonTransportError).to.be.an.instanceOf(ResponseError);
    expect(jsonTransportError.getDAPIAddress()).to.deep.equal(dapiAddress);
    expect(jsonTransportError.getCode()).to.equal(jsonRpcError.code);
    expect(jsonTransportError.getData()).to.deep.equal(jsonRpcError.data);
    expect(jsonTransportError.message).to.equal(error.message);
  });

  it('should return RetriableResponseError on JsonRpcError', () => {
    const jsonRpcError = {
      code: -32603,
      message: 'error message',
      data: {
        info: 'some data',
      },
    };

    const error = new JsonRpcError(requestInfo, jsonRpcError);

    const jsonTransportError = createJsonTransportError(error, dapiAddress);

    expect(jsonTransportError).to.be.an.instanceOf(RetriableResponseError);
    expect(jsonTransportError.getDAPIAddress()).to.deep.equal(dapiAddress);
    expect(jsonTransportError.getCode()).to.equal(jsonRpcError.code);
    expect(jsonTransportError.getData()).to.deep.equal(jsonRpcError.data);
    expect(jsonTransportError.message).to.equal(error.message);
  });

  it('should return ResponseError', () => {
    const error = new Error('Unknown error');
    error.code = 'UNKNOWN';

    const jsonTransportError = createJsonTransportError(error, dapiAddress);

    expect(jsonTransportError).to.be.an.instanceOf(ResponseError);
    expect(jsonTransportError.getDAPIAddress()).to.deep.equal(dapiAddress);
    expect(jsonTransportError.getCode()).to.equal(error.code);
    expect(jsonTransportError.getData()).to.deep.equal({});
    expect(jsonTransportError.message).to.equal(error.message);
  });

  it('should return RetriableResponseError', () => {
    const error = new Error('Aborted');
    error.code = 'ECONNABORTED';

    const jsonTransportError = createJsonTransportError(error, dapiAddress);

    expect(jsonTransportError).to.be.an.instanceOf(RetriableResponseError);
    expect(jsonTransportError.getDAPIAddress()).to.deep.equal(dapiAddress);
    expect(jsonTransportError.getCode()).to.equal(error.code);
    expect(jsonTransportError.getData()).to.deep.equal({});
    expect(jsonTransportError.message).to.equal(error.message);
  });
});
