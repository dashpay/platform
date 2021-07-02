const {
  v0: {
    GetTransactionRequest,
    GetTransactionResponse: ProtoGetTransactionResponse,
    CorePromiseClient,
  },
} = require('@dashevo/dapi-grpc');

const grpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');

const getTransactionFactory = require('../../../../../lib/methods/core/getTransaction/getTransactionFactory');
const GetTransactionResponse = require('../../../../../lib/methods/core/getTransaction/GetTransactionResponse');
const NotFoundError = require('../../../../../lib/methods/errors/NotFoundError');

describe('getTransactionFactory', () => {
  let getTransaction;
  let grpcTransportMock;
  let transaction;
  let blockHash;
  let height;
  let confirmations;
  let isChainLocked;
  let isInstantLocked;

  beforeEach(function beforeEach() {
    transaction = Buffer.from('transaction');
    blockHash = Buffer.from('blockHash');
    height = 42;
    confirmations = 3;
    isChainLocked = true;
    isInstantLocked = false;

    const response = new ProtoGetTransactionResponse();
    response.setTransaction(transaction);
    response.setBlockHash(blockHash);
    response.setHeight(height);
    response.setConfirmations(confirmations);
    response.setIsChainLocked(isChainLocked);
    response.setIsInstantLocked(isInstantLocked);

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

    expect(result).to.be.instanceof(GetTransactionResponse);
    expect(result.getTransaction()).to.deep.equal(transaction);
    expect(result.getBlockHash()).to.deep.equal(blockHash);
    expect(result.getConfirmations()).to.equal(confirmations);
    expect(result.getHeight()).to.equal(height);
    expect(result.isInstantLocked()).to.equal(isInstantLocked);
    expect(result.isChainLocked()).to.equal(isChainLocked);
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

    try {
      await getTransaction(id);
      expect.fail('should throw not found error');
    } catch (e) {
      expect(e).to.be.an.instanceOf(NotFoundError);
    }
  });

  it('should throw NotFoundError if transaction is not found', async () => {
    const error = new Error('Nothing found');
    error.code = grpcErrorCodes.NOT_FOUND;

    grpcTransportMock.request.throws(error);

    const id = '4f46066bd50cc2684484407696b7949e82bd906ea92c040f59a97cba47ed8176';

    try {
      await getTransaction(id);

      expect.fail('should throw NotFoundError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(NotFoundError);
    }

    const request = new GetTransactionRequest();
    request.setId(id);

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
