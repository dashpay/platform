const getMnListDiffFactory = require('../../../../lib/methods/core/getMnListDiffFactory');

const getMNListDiffsFixture = require('../../../../lib/test/fixtures/getMNListDiffsFixture');

describe('getMnListDiff', () => {
  let getMnListDiff;
  let jsonRpcTransportMock;
  let mnListDiff;
  let baseBlockHash;
  let blockHash;

  beforeEach(function beforeEach() {
    baseBlockHash = '0000047d24635e347be3aaaeb66c26be94901a2f962feccd4f95090191f208c1';
    blockHash = '000000000b0339e07bce8b3186a6a57a3c45d10e16c4bce18ef81b667bc822b2';
    mnListDiff = getMNListDiffsFixture();

    jsonRpcTransportMock = {
      request: this.sinon.stub().resolves(mnListDiff),
    };
    getMnListDiff = getMnListDiffFactory(jsonRpcTransportMock);
  });

  it('should return deterministic masternodelist diff', async () => {
    const options = {};

    const result = await getMnListDiff(baseBlockHash, blockHash, options);

    expect(result).to.deep.equal(mnListDiff);
    expect(jsonRpcTransportMock.request).to.be.calledOnceWithExactly(
      'getMnListDiff',
      { baseBlockHash, blockHash },
      options,
    );
  });
});
