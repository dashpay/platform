const getBlockHashFactory = require('../../../../lib/methods/core/getBlockHashFactory');

describe('getBlockHashFactory', () => {
  let getBlockHash;
  let jsonRpcTransport;
  let hash;

  beforeEach(function beforeEach() {
    hash = '000000000b0339e07bce8b3186a6a57a3c45d10e16c4bce18ef81b667bc822b2';

    jsonRpcTransport = {
      request: this.sinon.stub().resolves(hash),
    };
    getBlockHash = getBlockHashFactory(jsonRpcTransport);
  });

  it('should return best block hash', async () => {
    const options = {
      timeout: 1000,
    };

    const height = 1;

    const result = await getBlockHash(height, options);

    expect(result).to.deep.equal(hash);
    expect(jsonRpcTransport.request).to.be.calledOnceWithExactly(
      'getBlockHash',
      { height },
      options,
    );
  });
});
