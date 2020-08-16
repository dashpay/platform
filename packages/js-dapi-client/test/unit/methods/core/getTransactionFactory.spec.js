const {
  v0: {
    GetTransactionRequest,
    GetTransactionResponse,
    CorePromiseClient,
  },
} = require('@dashevo/dapi-grpc');

const grpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');

const getTransactionFactory = require('../../../../lib/methods/core/getTransactionFactory');

describe('getTransactionFactory', () => {
  let getTransaction;
  let grpcTransportMock;
  let transaction;

  beforeEach(function beforeEach() {
    transaction = Buffer.from('transaction');

    const response = new GetTransactionResponse();
    response.setTransaction(transaction);

    grpcTransportMock = {
      request: this.sinon.stub().resolves(response),
    };
    getTransaction = getTransactionFactory(grpcTransportMock);
  });

  it('should return transaction', async () => {
    const options = {
      timeout: 1000,
    };

    const id = '4f46066bd50cc2684484407696b7949e82bd906ea92c040f59a97cba47ed8176';

    const result = await getTransaction(id, options);

    const request = new GetTransactionRequest();
    request.setId(id);

    expect(result).to.be.instanceof(Buffer);
    expect(result).to.deep.equal(transaction);
    expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
      CorePromiseClient,
      'getTransaction',
      request,
      options,
    );
  });

  it('should return null if GRPC not found error has occurred', async () => {
    const error = new Error('Nothing found');
    error.code = grpcErrorCodes.NOT_FOUND;

    grpcTransportMock.request.throws(error);

    const id = '4f46066bd50cc2684484407696b7949e82bd906ea92c040f59a97cba47ed8176';

    const result = await getTransaction(id);

    const request = new GetTransactionRequest();
    request.setId(id);

    expect(result).to.equal(null);
    expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
      CorePromiseClient,
      'getTransaction',
      request,
      {},
    );
  });

  it('should return null if transaction is not found', async () => {
    const response = new GetTransactionResponse();
    grpcTransportMock.request.resolves(response);

    const id = '4f46066bd50cc2684484407696b7949e82bd906ea92c040f59a97cba47ed8176';

    const result = await getTransaction(id);

    const request = new GetTransactionRequest();
    request.setId(id);

    expect(result).to.equal(null);
    expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
      CorePromiseClient,
      'getTransaction',
      request,
      {},
    );
  });

  it('should throw unknown error', async () => {
    const error = new Error('Unknown found');

    grpcTransportMock.request.throws(error);

    const id = '4f46066bd50cc2684484407696b7949e82bd906ea92c040f59a97cba47ed8176';

    const request = new GetTransactionRequest();
    request.setId(id);

    try {
      await getTransaction(id);

      expect.fail('should throw unknown error');
    } catch (e) {
      expect(e).to.deep.equal(error);
      expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
        CorePromiseClient,
        'getTransaction',
        request,
        {},
      );
    }
  });
});
