const getBestBlockHashFactory = require('../../../../lib/methods/core/getBestBlockHashFactory');

describe('getBestBlockHashFactory', () => {
  let getBestBlockHash;
  let jsonRpcTransport;
  let bestBlockHash;

  beforeEach(function beforeEach() {
    bestBlockHash = '000000000b0339e07bce8b3186a6a57a3c45d10e16c4bce18ef81b667bc822b2';

    jsonRpcTransport = {
      request: this.sinon.stub().resolves(bestBlockHash),
    };
    getBestBlockHash = getBestBlockHashFactory(jsonRpcTransport);
  });

  it('should return best block hash', async () => {
    const options = {
      timeout: 1000,
    };

    const result = await getBestBlockHash(options);

    expect(result).to.deep.equal(bestBlockHash);
    expect(jsonRpcTransport.request).to.be.calledOnceWithExactly(
      'getBestBlockHash',
      {},
      options,
    );
  });
});
