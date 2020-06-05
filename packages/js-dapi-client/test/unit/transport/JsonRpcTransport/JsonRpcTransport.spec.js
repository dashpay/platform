const JsonRpcTransport = require('../../../../lib/transport/JsonRpcTransport/JsonRpcTransport');
const DAPIAddress = require('../../../../lib/dapiAddressProvider/DAPIAddress');

const MaxRetriesReachedError = require('../../../../lib/transport/errors/MaxRetriesReachedError');
const NoAvailableAddressesForRetry = require('../../../../lib/transport/errors/NoAvailableAddressesForRetry');

describe('JsonRpcTransport', () => {
  let jsonRpcTransport;
  let globalOptions;
  let createDAPIAddressProviderFromOptionsMock;
  let dapiAddressProviderMock;
  let dapiAddress;
  let host;
  let requestJsonRpcMock;

  beforeEach(function beforeEach() {
    host = '127.0.0.1';
    dapiAddress = new DAPIAddress(host);

    globalOptions = {
      retries: 0,
    };

    dapiAddressProviderMock = {
      getLiveAddress: this.sinon.stub().resolves(dapiAddress),
      hasLiveAddresses: this.sinon.stub().resolves(false),
    };

    createDAPIAddressProviderFromOptionsMock = this.sinon.stub().returns(null);

    requestJsonRpcMock = this.sinon.stub();

    jsonRpcTransport = new JsonRpcTransport(
      createDAPIAddressProviderFromOptionsMock,
      requestJsonRpcMock,
      dapiAddressProviderMock,
      globalOptions,
    );
  });

  describe('#request', () => {
    let method;
    let params;
    let options;
    let data;

    beforeEach(() => {
      params = {
        data: 'some params',
      };
      options = {
        timeout: 1000,
      };
      method = 'method';
      data = 'result';

      requestJsonRpcMock.resolves(data);
    });

    it('should make a request', async () => {
      const receivedData = await jsonRpcTransport.request(
        method,
        params,
        options,
      );

      expect(receivedData).to.equal(data);
      expect(createDAPIAddressProviderFromOptionsMock).to.be.calledOnceWithExactly(options);
      expect(jsonRpcTransport.lastUsedAddress).to.deep.equal(dapiAddress);
      expect(requestJsonRpcMock).to.be.calledOnceWithExactly(
        dapiAddress.getHost(),
        dapiAddress.getHttpPort(),
        method,
        params,
        { timeout: options.timeout },
      );
    });

    it('should throw unknown error', async () => {
      const error = new Error('Unknown error');
      requestJsonRpcMock.throws(error);

      try {
        await jsonRpcTransport.request(
          method,
          params,
        );

        expect.fail('should throw error');
      } catch (e) {
        expect(e).to.deep.equal(error);
        expect(createDAPIAddressProviderFromOptionsMock).to.be.calledOnceWithExactly({});
        expect(jsonRpcTransport.lastUsedAddress).to.deep.equal(dapiAddress);
        expect(requestJsonRpcMock).to.be.calledOnceWithExactly(
          dapiAddress.getHost(),
          dapiAddress.getHttpPort(),
          method,
          params,
          {},
        );
      }
    });

    it('should throw MaxRetriesReachedError', async () => {
      const error = new Error('Internal error');
      error.code = 'ECONNABORTED';
      requestJsonRpcMock.throws(error);

      try {
        await jsonRpcTransport.request(
          method,
        );

        expect.fail('should throw MaxRetriesReachedError');
      } catch (e) {
        expect(e).to.be.an.instanceof(MaxRetriesReachedError);
        expect(e.getError()).to.equal(error);
        expect(createDAPIAddressProviderFromOptionsMock).to.be.calledOnceWithExactly({});
        expect(jsonRpcTransport.lastUsedAddress).to.deep.equal(dapiAddress);
        expect(requestJsonRpcMock).to.be.calledOnceWithExactly(
          dapiAddress.getHost(),
          dapiAddress.getHttpPort(),
          method,
          {},
          {},
        );
      }
    });

    it('should throw NoAvailableAddressesForRetry error', async () => {
      const error = new Error('Internal error');
      error.code = 'ECONNABORTED';
      requestJsonRpcMock.throws(error);

      options.retries = 1;

      try {
        await jsonRpcTransport.request(
          method,
          params,
          options,
        );

        expect.fail('should throw NoAvailableAddressesForRetry');
      } catch (e) {
        expect(e).to.be.an.instanceof(NoAvailableAddressesForRetry);
        expect(e.getError()).to.equal(error);
        expect(createDAPIAddressProviderFromOptionsMock).to.be.calledOnceWithExactly(options);
        expect(jsonRpcTransport.lastUsedAddress).to.deep.equal(dapiAddress);
        expect(requestJsonRpcMock).to.be.calledOnceWithExactly(
          dapiAddress.getHost(),
          dapiAddress.getHttpPort(),
          method,
          params,
          { timeout: options.timeout },
        );
      }
    });

    it('should retry the request if a connection aborted error has thrown', async () => {
      dapiAddressProviderMock.hasLiveAddresses.resolves(true);
      const error = new Error('Internal error');
      error.code = 'ECONNABORTED';
      requestJsonRpcMock.onCall(0).throws(error);

      options.retries = 1;
      const receivedData = await jsonRpcTransport.request(
        method,
        params,
        options,
      );

      expect(receivedData).to.equal(data);
      expect(createDAPIAddressProviderFromOptionsMock).to.be.calledTwice();
      expect(jsonRpcTransport.lastUsedAddress).to.deep.equal(dapiAddress);
      expect(requestJsonRpcMock).to.be.calledTwice();
    });

    it('should retry the request if a connection refused error has thrown', async () => {
      dapiAddressProviderMock.hasLiveAddresses.resolves(true);
      const error = new Error('Internal error');
      error.code = 'ECONNREFUSED';
      requestJsonRpcMock.onCall(0).throws(error);

      options.retries = 1;
      const receivedData = await jsonRpcTransport.request(
        method,
        params,
        options,
      );

      expect(receivedData).to.equal(data);
      expect(createDAPIAddressProviderFromOptionsMock).to.be.calledTwice();
      expect(jsonRpcTransport.lastUsedAddress).to.deep.equal(dapiAddress);
      expect(requestJsonRpcMock).to.be.calledTwice();
    });

    it('should retry the request if error with code -32603 has thrown', async () => {
      dapiAddressProviderMock.hasLiveAddresses.resolves(true);
      const error = new Error('Internal error');
      error.code = -32603;
      requestJsonRpcMock.onCall(0).throws(error);

      options.retries = 1;
      const receivedData = await jsonRpcTransport.request(
        method,
        params,
        options,
      );

      expect(receivedData).to.equal(data);
      expect(createDAPIAddressProviderFromOptionsMock).to.be.calledTwice();
      expect(jsonRpcTransport.lastUsedAddress).to.deep.equal(dapiAddress);
      expect(requestJsonRpcMock).to.be.calledTwice();
    });

    it('should throw unknown error if error with code -32000 has thrown', async () => {
      dapiAddressProviderMock.hasLiveAddresses.resolves(true);
      const error = new Error('Internal error');
      error.code = -32000;
      requestJsonRpcMock.onCall(0).throws(error);

      try {
        await jsonRpcTransport.request(
          method,
          params,
          options,
        );

        expect.fail('should throw error');
      } catch (e) {
        expect(e).to.deep.equal(error);
        expect(createDAPIAddressProviderFromOptionsMock).to.be.calledOnceWithExactly(options);
        expect(jsonRpcTransport.lastUsedAddress).to.deep.equal(dapiAddress);
        expect(requestJsonRpcMock).to.be.calledOnceWithExactly(
          dapiAddress.getHost(),
          dapiAddress.getHttpPort(),
          method,
          params,
          { timeout: options.timeout },
        );
      }
    });
  });

  describe('#getLastUsedAddress', () => {
    it('should return lastUsedAddress', async () => {
      jsonRpcTransport.lastUsedAddress = dapiAddress;

      const lastUsedAddress = jsonRpcTransport.getLastUsedAddress();

      expect(lastUsedAddress).to.deep.equal(dapiAddress);
    });
  });
});
