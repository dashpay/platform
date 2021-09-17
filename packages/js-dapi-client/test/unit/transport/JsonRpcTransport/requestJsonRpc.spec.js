const axios = require('axios');
const requestJsonRpc = require('../../../../lib/transport/JsonRpcTransport/requestJsonRpc');
const JsonRpcError = require('../../../../lib/transport/JsonRpcTransport/errors/JsonRpcError');
const WrongHttpCodeError = require('../../../../lib/transport/JsonRpcTransport/errors/WrongHttpCodeError');

describe('requestJsonRpc', () => {
  let host;
  let port;
  let timeout;
  let params;

  beforeEach(function beforeEach() {
    host = 'localhost';
    port = 80;
    params = { data: 'test' };
    timeout = 1000;

    const options = { timeout };

    const url = `http://${host}:${port}`;
    const payload = {
      jsonrpc: '2.0',
      params,
      id: 1,
    };

    const axiosStub = this.sinon.stub(axios, 'post');

    axiosStub
      .withArgs(
        url,
        { ...payload, method: 'shouldPass' },
        options,
      )
      .resolves({ status: 200, data: { result: 'passed', error: null } });

    axiosStub
      .withArgs(
        `https://${host}`,
        { ...payload, method: 'httpsRequest' },
        options,
      )
      .resolves({ status: 200, data: { result: 'passed', error: null } });

    axiosStub
      .withArgs(
        url,
        { ...payload, method: 'wrongData' },
        options,
      )
      .resolves({ status: 400, data: { result: null, error: { message: 'Wrong data' } }, statusMessage: 'Status message' });

    axiosStub
      .withArgs(
        url,
        { ...payload, method: 'invalidData' },
        options,
      )
      .resolves({ status: 200, data: { result: null, error: { message: 'invalid data' } } });

    axiosStub
      .withArgs(
        url,
        { ...payload, method: 'errorData' },
        { timeout: undefined },
      )
      .resolves({ status: 200, data: { result: null, error: { message: 'Invalid data for error.data', data: 'additional data here', code: -1 } } });
  });

  afterEach(() => {
    axios.post.restore();
  });

  it('should make rpc request and return result', async () => {
    const result = await requestJsonRpc(
      host,
      port,
      'shouldPass',
      params,
      { timeout },
    );

    expect(result).to.equal('passed');
  });

  it('should make https rpc request and return result', async () => {
    port = 443;

    const result = await requestJsonRpc(
      host,
      port,
      'httpsRequest',
      params,
      { timeout },
    );

    expect(result).to.equal('passed');
  });

  it('should throw WrongHttpCodeError if response status is not 200', async () => {
    const method = 'wrongData';
    const options = { timeout };

    try {
      await requestJsonRpc(
        host,
        port,
        method,
        params,
        options,
      );

      expect.fail('should throw error');
    } catch (e) {
      expect(e).to.be.an.instanceOf(WrongHttpCodeError);
      expect(e.message).to.equal('DAPI JSON RPC wrong http code: Status message');
      expect(e.getStatusCode()).to.equal(400);
      expect(e.getRequestInfo()).to.deep.equal({
        host,
        port,
        method,
        params,
        options,
      });
    }
  });

  it('should throw error if there is an error object in the response body', async () => {
    try {
      await requestJsonRpc(
        host,
        port,
        'invalidData',
        params,
        { timeout },
      );

      expect.fail('should throw error');
    } catch (e) {
      expect(e.message).to.equal('invalid data');
    }
  });

  it('should throw error if there is an error object with data in the response body', async () => {
    const method = 'errorData';

    try {
      await requestJsonRpc(
        host,
        port,
        method,
        params,
      );

      expect.fail('should throw error');
    } catch (e) {
      expect(e).to.be.an.instanceof(JsonRpcError);
      expect(e.message).to.equal('Invalid data for error.data');
      expect(e.getRequestInfo()).to.deep.equal({
        host,
        port,
        method,
        params,
        options: {},
      });
      expect(e.getMessage()).to.equal('Invalid data for error.data');
      expect(e.getData()).to.equal('additional data here');
      expect(e.getCode()).to.equal(-1);
    }
  });
});
