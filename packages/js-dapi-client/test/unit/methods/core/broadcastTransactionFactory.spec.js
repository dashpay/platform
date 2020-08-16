const {
  v0: {
    CorePromiseClient,
    BroadcastTransactionRequest,
    BroadcastTransactionResponse,
  },
} = require('@dashevo/dapi-grpc');

const broadcastTransactionFactory = require(
  '../../../../lib/methods/core/broadcastTransactionFactory',
);

describe('broadcastTransactionFactory', () => {
  let broadcastTransaction;
  let grpcTransportMock;
  let transaction;
  let id;

  beforeEach(function beforeEach() {
    grpcTransportMock = {
      request: this.sinon.stub(),
    };

    broadcastTransaction = broadcastTransactionFactory(
      grpcTransportMock,
    );

    transaction = Buffer.from('transaction');
    id = '4f46066bd50cc2684484407696b7949e82bd906ea92c040f59a97cba47ed8176';
  });

  it('should return transaction id', async () => {
    const response = new BroadcastTransactionResponse();
    response.setTransactionId(id);
    grpcTransportMock.request.resolves(response);

    const options = {
      allowHighFees: false,
    };

    const result = await broadcastTransaction(
      transaction,
      options,
    );

    const request = new BroadcastTransactionRequest();
    request.setTransaction(transaction);
    request.setAllowHighFees(options.allowHighFees);
    request.setBypassLimits(false);

    expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
      CorePromiseClient,
      'broadcastTransaction',
      request,
      options,
    );
    expect(result).to.equal(id);
  });
});
