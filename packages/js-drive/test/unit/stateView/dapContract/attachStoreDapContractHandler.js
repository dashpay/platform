const getTransitionHeaderFixtures = require('../../../../lib/test/fixtures/getTransitionHeaderFixtures');
const attachStoreDapContractHandler = require('../../../../lib/stateView/dapContract/attachStoreDapContractHandler');

describe('attachStoreDapContractHandler', () => {
  let stHeadersReader;
  let storeDapContract;

  beforeEach(function beforeEach() {
    const header = getTransitionHeaderFixtures()[0];
    stHeadersReader = {
      on: (topic, fn) => fn(header),
    };
    storeDapContract = this.sinon.stub();
    attachStoreDapContractHandler(stHeadersReader, storeDapContract);
  });

  it('should call attachStoreDapContractHandler on new block header', () => {
    expect(storeDapContract).to.be.calledOnce();
  });
});
