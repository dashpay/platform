const TransactionsFilterStreamPromiseClient = require('../../../../clients/nodejs/TransactionsFilterStreamPromiseClient');

describe('TransactionsFilterStreamPromiseClient', () => {
  let transactionsFilterStreamPromiseClient;
  let request;
  let response;

  beforeEach(function main() {
    request = 'test request';
    response = 'test response';

    transactionsFilterStreamPromiseClient = new TransactionsFilterStreamPromiseClient('localhost');
    transactionsFilterStreamPromiseClient.client = {
      subscribeToTransactionsWithProofs: this.sinon.stub().resolves(response),
    };
  });

  it('should subscribe to transactions with proofs', async () => {
    const result = await transactionsFilterStreamPromiseClient
      .subscribeToTransactionsWithProofs(request);

    expect(result).to.equal(response);
    expect(transactionsFilterStreamPromiseClient.client.subscribeToTransactionsWithProofs)
      .to.be.calledOnceWith(request);
  });

  it('should throw an error when metadata is not an object', async () => {
    try {
      transactionsFilterStreamPromiseClient.subscribeToTransactionsWithProofs({}, 'metadata');

      expect.fail('Error was not thrown');
    } catch (e) {
      expect(e.message).to.equal('metadata must be an object');
    }
  });
});
