const Emitter = require('emittery');
const getTransitionHeaderFixtures = require('../../../../lib/test/fixtures/getTransitionHeaderFixtures');
const attachStateViewHandlers = require('../../../../lib/stateView/dapContract/attachStateViewHandlers');

describe('attachStateViewHandlers', () => {
  let stHeadersReader;
  let storeDapContract;
  let dropMongoDatabasesWithPrefixStub;

  beforeEach(function beforeEach() {
    class STHeadersReader extends Emitter {}
    stHeadersReader = new STHeadersReader();
    storeDapContract = this.sinon.stub();
    dropMongoDatabasesWithPrefixStub = this.sinon.stub();
    attachStateViewHandlers(
      stHeadersReader,
      storeDapContract,
      dropMongoDatabasesWithPrefixStub,
    );
  });

  it('should call attachStateViewHandlers on new block header', async () => {
    const header = getTransitionHeaderFixtures()[0];
    await stHeadersReader.emitSerial('header', header);
    expect(storeDapContract).to.be.calledOnce();
  });

  it('should call dropMongoDatabasesWithPrefix on reset event', async () => {
    await stHeadersReader.emit('reset');
    expect(dropMongoDatabasesWithPrefixStub).to.be.calledOnce();
  });
});
