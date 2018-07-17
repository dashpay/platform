const Emitter = require('emittery');
const getTransitionHeaderFixtures = require('../../../lib/test/fixtures/getTransitionHeaderFixtures');
const attachStateViewHandlers = require('../../../lib/stateView/attachStateViewHandlers');

describe('attachStateViewHandlers', () => {
  let stHeadersReader;
  let applyStateTransition;
  let dropMongoDatabasesWithPrefixStub;

  beforeEach(function beforeEach() {
    class STHeadersReader extends Emitter {}
    stHeadersReader = new STHeadersReader();
    applyStateTransition = this.sinon.stub();
    dropMongoDatabasesWithPrefixStub = this.sinon.stub();
    attachStateViewHandlers(
      stHeadersReader,
      applyStateTransition,
      dropMongoDatabasesWithPrefixStub,
    );
  });

  it('should call attachStateViewHandlers on new block header', async () => {
    const header = getTransitionHeaderFixtures()[0];
    await stHeadersReader.emitSerial('header', { header });
    expect(applyStateTransition).to.be.calledOnce();
  });

  it('should call dropMongoDatabasesWithPrefix on reset event', async () => {
    await stHeadersReader.emit('reset');
    expect(dropMongoDatabasesWithPrefixStub).to.be.calledOnce();
  });
});
