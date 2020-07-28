const {
  server: {
    error: {
      InvalidArgumentGrpcError,
    },
  },
} = require('@dashevo/grpc-common');

const { Transaction } = require('@dashevo/dashcore-lib');

const {
  BroadcastTransactionResponse,
} = require('@dashevo/dapi-grpc');

const broadcastTransactionHandlerFactory = require('../../../../../lib/grpcServer/handlers/core/broadcastTransactionHandlerFactory');

const GrpcCallMock = require('../../../../../lib/test/mock/GrpcCallMock');

describe('broadcastTransactionHandlerFactory', () => {
  let call;
  let insightAPIMock;
  let request;
  let serializedTransaction;
  let transactionId;
  let broadcastTransactionHandler;

  beforeEach(function beforeEach() {
    const rawTransaction = '0300000001086a3640a4a88a85d5720ecb69a93d0aef1cfa759d1242835e4abaf4168b924d000000006b483045022100ed608a9742913c94e057798297a6a96ed40c41dc61209e6887df51ea5755234802207f5733ef592f3df59bc6d39749ac5f1a771b32263ce16982b1aacc80ff1358cd012103323aa9dd83ba005b1b1e61b36cba27c2e0f64bacb57c34243fc7ef2751fff6edffffffff021027000000000000166a1481b21f3898087a0d1905140c7db8d7db00acd13954a09a3b000000001976a91481b21f3898087a0d1905140c7db8d7db00acd13988ac00000000';

    serializedTransaction = new Transaction(rawTransaction).toBuffer();

    transactionId = 'id';

    request = {
      getTransaction: this.sinon.stub().returns(serializedTransaction),
    };

    call = new GrpcCallMock(this.sinon, request);

    insightAPIMock = {
      sendTransaction: this.sinon.stub().resolves(transactionId),
    };

    broadcastTransactionHandler = broadcastTransactionHandlerFactory(insightAPIMock);
  });

  it('should return valid result', async () => {
    const result = await broadcastTransactionHandler(call);

    expect(result).to.be.an.instanceOf(BroadcastTransactionResponse);
    expect(result.getTransactionId()).to.equal(transactionId);
    expect(insightAPIMock.sendTransaction).to.be.calledOnceWith(serializedTransaction.toString('hex'));
  });

  it('should throw InvalidArgumentGrpcError error if transaction is not specified', async () => {
    serializedTransaction = null;
    request.getTransaction.returns(serializedTransaction);

    try {
      await broadcastTransactionHandler(call);

      expect.fail('should thrown InvalidArgumentGrpcError error');
    } catch (e) {
      expect(e).to.be.instanceOf(InvalidArgumentGrpcError);
      expect(e.getMessage()).to.equal('transaction is not specified');
      expect(insightAPIMock.sendTransaction).to.be.not.called();
    }
  });

  it('should throw InvalidArgumentGrpcError error if transaction is not valid', async () => {
    serializedTransaction = '03000000011846a52a9e766cbb0a6153bb78af9858cc71070aea44cd8282ba8e5c5de7331b000000006a4730440220267c9903049b8962f67ed7809e0f5cf32324d999b7a85c9d883298600bb880ab022048478303e2281e26cefa496e7b3aeac0c5cccbe6e0429adf8532438f996c4a0c012103fe92ef7d837791caaf44be835a1782b4b1f0865c4c6ae73a4a92b14c8a37cc78ffffffff0000000000';
    request.getTransaction.returns(serializedTransaction);

    try {
      await broadcastTransactionHandler(call);

      expect.fail('should thrown InvalidArgumentGrpcError error');
    } catch (e) {
      expect(e).to.be.instanceOf(InvalidArgumentGrpcError);
      expect(e.getMessage()).to.be.a('string').and.satisfy(msg => msg.startsWith('invalid transaction:'));
      expect(insightAPIMock.sendTransaction).to.be.not.called();
    }
  });

  it('should throw InvalidArgumentGrpcError error if transaction cannot be decoded', async () => {
    serializedTransaction = new Uint8Array(Buffer.from('invalid data'));
    request.getTransaction.returns(serializedTransaction);

    try {
      await broadcastTransactionHandler(call);

      expect.fail('should thrown InvalidArgumentGrpcError error');
    } catch (e) {
      expect(e).to.be.instanceOf(InvalidArgumentGrpcError);
      expect(e.getMessage()).to.be.a('string').and.satisfy(msg => msg.startsWith('invalid transaction:'));
      expect(insightAPIMock.sendTransaction).to.be.not.called();
    }
  });
});
