const JsonRpcTransport = require('../../../../lib/transport/JsonRpcTransport/JsonRpcTransport');
const DAPIAddress = require('../../../../lib/dapiAddressProvider/DAPIAddress');

const MaxRetriesReachedError = require('../../../../lib/transport/errors/response/MaxRetriesReachedError');
const NoAvailableAddressesForRetryError = require('../../../../lib/transport/errors/response/NoAvailableAddressesForRetryError');
const NoAvailableAddressesError = require('../../../../lib/transport/errors/NoAvailableAddressesError');
const ResponseError = require('../../../../lib/transport/errors/response/ResponseError');
const JsonRpcError = require('../../../../lib/transport/JsonRpcTransport/errors/JsonRpcError');
const RetriableResponseError = require('../../../../lib/transport/errors/response/RetriableResponseError');

describe('JsonRpcTransport', () => {
  let jsonRpcTransport;
  let globalOptions;
  let createDAPIAddressProviderFromOptionsMock;
  let dapiAddressProviderMock;
  let dapiAddress;
  let host;
  let requestJsonRpcMock;
  let createJsonTransportErrorMock;

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

    createJsonTransportErrorMock = this.sinon.stub();

    jsonRpcTransport = new JsonRpcTransport(
      createDAPIAddressProviderFromOptionsMock,
      requestJsonRpcMock,
      dapiAddressProviderMock,
      createJsonTransportErrorMock,
      globalOptions,
    );
  });

  describe('#request', () => {
    let method;
    let params;
    let options;
    let data;
    let requestInfo;
    let jsonRpcErrorObject;

    beforeEach(() => {
      params = {
        data: 'some params',
      };
      options = {
        timeout: 1000,
      };
      method = 'method';
      data = 'result';

      requestInfo = {};

      jsonRpcErrorObject = {
        code: 1,
        message: 'hello',
        data: {},
      };

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

    it('should throw NoAvailableAddresses if there is no available addresses', async () => {
      dapiAddressProviderMock.getLiveAddress.resolves(null);

      try {
        await jsonRpcTransport.request(
          method,
        );

        expect.fail('should throw NoAvailableAddresses');
      } catch (e) {
        expect(e).to.be.an.instanceof(NoAvailableAddressesError);
        expect(requestJsonRpcMock).to.not.be.called();
      }
    });

    it('should throw non-retriable response error', async () => {
      const error = new JsonRpcError(requestInfo, jsonRpcErrorObject);

      requestJsonRpcMock.throws(error);

      const responseError = new ResponseError(
        error.getCode(),
        error.getMessage(),
        error.getData(),
        dapiAddress,
      );

      createJsonTransportErrorMock.returns(responseError);

      try {
        await jsonRpcTransport.request(
          method,
          params,
        );

        expect.fail('should throw ResponseError');
      } catch (e) {
        expect(e).to.equal(responseError);

        expect(createJsonTransportErrorMock).to.be.calledOnceWithExactly(error, dapiAddress);

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
      const error = new JsonRpcError(requestInfo, jsonRpcErrorObject);

      requestJsonRpcMock.throws(error);

      const responseError = new RetriableResponseError(
        error.getCode(),
        error.getMessage(),
        error.getData(),
        dapiAddress,
      );

      createJsonTransportErrorMock.returns(responseError);

      options.retries = 0;

      try {
        await jsonRpcTransport.request(
          method,
        );

        expect.fail('should throw MaxRetriesReachedError');
      } catch (e) {
        expect(e).to.be.an.instanceof(MaxRetriesReachedError);
        expect(e.getCause()).to.equal(responseError);

        expect(createJsonTransportErrorMock).to.be.calledOnceWithExactly(error, dapiAddress);

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
      const error = new JsonRpcError(requestInfo, jsonRpcErrorObject);

      requestJsonRpcMock.throws(error);

      const responseError = new RetriableResponseError(
        error.getCode(),
        error.getMessage(),
        error.getData(),
        dapiAddress,
      );

      createJsonTransportErrorMock.returns(responseError);

      options.retries = 1;

      try {
        await jsonRpcTransport.request(
          method,
          params,
          options,
        );

        expect.fail('should throw NoAvailableAddressesForRetry');
      } catch (e) {
        expect(e).to.be.an.instanceof(NoAvailableAddressesForRetryError);
        expect(e.getCause()).to.equal(responseError);

        expect(createJsonTransportErrorMock).to.be.calledOnceWithExactly(error, dapiAddress);

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
