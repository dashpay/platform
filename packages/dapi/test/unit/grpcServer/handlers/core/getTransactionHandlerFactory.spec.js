const {
  server: {
    error: {
      InvalidArgumentGrpcError,
      NotFoundGrpcError,
    },
  },
} = require('@dashevo/grpc-common');

const {
  v0: {
    GetTransactionResponse,
  },
} = require('@dashevo/dapi-grpc');

const { Transaction } = require('@dashevo/dashcore-lib');

const getTransactionHandlerFactory = require('../../../../../lib/grpcServer/handlers/core/getTransactionHandlerFactory');

const GrpcCallMock = require('../../../../../lib/test/mock/GrpcCallMock');

describe('getTransactionHandlerFactory', () => {
  let call;
  let request;
  let id;
  let rawTransactionFixture;
  let getTransactionHandler;
  let coreRPCClientMock;

  beforeEach(function beforeEach() {
    id = 'id';
    rawTransactionFixture = '0200000001d3145639d750ce104d740f7b2bb46381202e4798f9eb678cb361467195aa5b96000000006a4730440220156ea3d61ea7dce612a1608beed374138c8bf58aad7d76292d39d648e2b1346b022051f28f100bfa9d0dae0e2d8d76d00007f32611c8d69d49203f059ffc3c2fac58012102c9228c1cd1e778062de766376580e9dbeb301a4aa2cb0c535ada5a58f6ec5532ffffffff018dec9f00000000001976a914bf7f49e8e8c8aa0fcf1af8e53e117d25db30207288ac00000000';

    request = {
      getId: this.sinon.stub().returns(id),
    };

    call = new GrpcCallMock(this.sinon, request);

    coreRPCClientMock = {
      getRawTransaction: this.sinon.stub().resolves({
        hex: rawTransactionFixture,
        blockhash: Buffer.alloc(1, 32).toString('hex'),
        height: 42,
        confirmations: 3,
        instantlock_internal: true,
        chainlock: false,
      }),
    };

    getTransactionHandler = getTransactionHandlerFactory(coreRPCClientMock);
  });

  it('should return valid result', async () => {
    const result = await getTransactionHandler(call);

    expect(result).to.be.an.instanceOf(GetTransactionResponse);

    const transactionSerialized = result.getTransaction();

    expect(transactionSerialized).to.be.an.instanceOf(Buffer);

    const returnedTransaction = new Transaction(transactionSerialized);

    expect(returnedTransaction.toString()).to.deep.equal(rawTransactionFixture);
    expect(coreRPCClientMock.getRawTransaction).to.be.calledOnceWith(id);
    expect(result.getBlockHash()).to.deep.equal(Buffer.alloc(1, 32));
    expect(result.getHeight()).to.equal(42);
    expect(result.getConfirmations()).to.equal(3);
    expect(result.getIsInstantLocked()).to.be.true();
    expect(result.getIsChainLocked()).to.be.false();
  });

  it('should throw InvalidArgumentGrpcError error if id is not specified', async () => {
    id = null;
    request.getId.returns(id);

    try {
      await getTransactionHandler(call);

      expect.fail('should thrown InvalidArgumentGrpcError error');
    } catch (e) {
      expect(e).to.be.instanceOf(InvalidArgumentGrpcError);
      expect(e.getMessage()).to.equal('id is not specified');
      expect(coreRPCClientMock.getRawTransaction).to.be.not.called();
    }
  });

  it('should throw NotFoundGrpcError if transaction is not found', async () => {
    const error = new Error();
    error.code = -5;
    coreRPCClientMock.getRawTransaction.throws(error);

    try {
      await getTransactionHandler(call);

      expect.fail('should thrown InvalidArgumentGrpcError error');
    } catch (e) {
      expect(e).to.be.instanceOf(NotFoundGrpcError);
      expect(e.getMessage()).to.equal('Transaction not found');
      expect(coreRPCClientMock.getRawTransaction).to.be.calledOnceWith(id);
    }
  });
});
