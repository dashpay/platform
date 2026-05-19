const requestJsonRpc = require('../../../../lib/transport/JsonRpcTransport/requestJsonRpc');
const JsonRpcError = require('../../../../lib/transport/JsonRpcTransport/errors/JsonRpcError');
const WrongHttpCodeError = require('../../../../lib/transport/JsonRpcTransport/errors/WrongHttpCodeError');

describe('requestJsonRpc', () => {
  let protocol;
  let host;
  let port;
  let timeout;
  let params;
  let selfSigned;

  beforeEach(function beforeEach() {
    protocol = 'http';
    host = 'localhost';
    port = 80;
    params = { data: 'test' };
    timeout = 1000;
    selfSigned = false;

    // eslint-disable-next-line
    this.sinon.stub(globalThis, 'fetch');
  });

  afterEach(() => {
    // eslint-disable-next-line
    fetch.restore();
  });

  it('should make rpc request and return result', async () => {
    // eslint-disable-next-line
    fetch.resolves(new Response(
      JSON.stringify({ result: 'passed', error: null }),
      {
        status: 200,
      },
    ));

    const result = await requestJsonRpc(
      protocol,
      host,
      port,
      selfSigned,
      'shouldPass',
      params,
      { timeout },
    );

    expect(result).to.equal('passed');
  });

  it('should make https rpc request and return result', async () => {
    protocol = 'https';

    // eslint-disable-next-line
    fetch.resolves(new Response(
      JSON.stringify({ result: 'passed', error: null }),
      {
        status: 200,
      },
    ));

    const result = await requestJsonRpc(
      protocol,
      host,
      port,
      selfSigned,
      'httpsRequest',
      params,
      { timeout },
    );

    expect(result).to.equal('passed');
  });

  it('should pass an undici Agent that skips TLS verification when selfSigned is true', async () => {
    protocol = 'https';
    selfSigned = true;

    // eslint-disable-next-line
    fetch.resolves(new Response(
      JSON.stringify({ result: 'passed', error: null }),
      {
        status: 200,
      },
    ));

    const result = await requestJsonRpc(
      protocol,
      host,
      port,
      selfSigned,
      'httpsRequest',
      params,
      { timeout },
    );

    expect(result).to.equal('passed');

    // Verify fetch was actually given a dispatcher that disables TLS verification.
    // Without this, the selfSigned flag would be silently inert in Node.
    // eslint-disable-next-line
    const [, requestOptions] = fetch.firstCall.args;
    expect(requestOptions.dispatcher).to.exist();
  });

  it('should not pass a dispatcher when selfSigned is false', async () => {
    protocol = 'https';
    selfSigned = false;

    // eslint-disable-next-line
    fetch.resolves(new Response(
      JSON.stringify({ result: 'passed', error: null }),
      {
        status: 200,
      },
    ));

    await requestJsonRpc(
      protocol,
      host,
      port,
      selfSigned,
      'httpsRequest',
      params,
      { timeout },
    );

    // eslint-disable-next-line
    const [, requestOptions] = fetch.firstCall.args;
    expect(requestOptions.dispatcher).to.equal(undefined);
  });

  it('should throw WrongHttpCodeError if response status is not 200', async () => {
    const method = 'wrongData';
    const options = { timeout };

    // eslint-disable-next-line
    fetch.resolves(new Response(
      JSON.stringify({ result: null, error: { message: 'Wrong data' } }),
      {
        status: 400,
        statusText: 'Status message',
      },
    ));

    try {
      await requestJsonRpc(
        protocol,
        host,
        port,
        selfSigned,
        method,
        params,
        options,
      );

      expect.fail('should throw error');
    } catch (e) {
      expect(e).to.be.an.instanceOf(WrongHttpCodeError);
      expect(e.message).to.equal('DAPI JSON RPC wrong http code: Status message');
      expect(e.getCode()).to.equal(400);
      expect(e.getRequestInfo()).to.deep.equal({
        protocol,
        host,
        port,
        selfSigned,
        method,
        params,
        options,
      });
    }
  });

  it('should throw error if there is an error object in the response body', async () => {
    try {
      // eslint-disable-next-line
      fetch.resolves(new Response(
        JSON.stringify({ result: null, error: { message: 'invalid data' } }),
        {
          status: 200,
          statusText: 'Status message',
        },
      ));

      await requestJsonRpc(
        protocol,
        host,
        port,
        selfSigned,
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
      // eslint-disable-next-line
      fetch.resolves(new Response(
        JSON.stringify({ result: null, error: { message: 'Invalid data for error.data', data: 'additional data here', code: -1 } }),
        {
          status: 200,
          statusText: 'Status message',
        },
      ));

      await requestJsonRpc(
        protocol,
        host,
        port,
        selfSigned,
        method,
        params,
      );

      expect.fail('should throw error');
    } catch (e) {
      expect(e).to.be.an.instanceof(JsonRpcError);
      expect(e.message).to.equal('Invalid data for error.data');
      expect(e.getRequestInfo()).to.deep.equal({
        protocol,
        host,
        port,
        selfSigned,
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
