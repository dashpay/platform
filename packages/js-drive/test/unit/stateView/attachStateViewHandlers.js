const ReaderMediator = require('../../../lib/blockchain/reader/BlockchainReaderMediator');

const BlockchainReaderMediatorMock = require('../../../lib/test/mock/BlockchainReaderMediatorMock');

const attachStateViewHandlers = require('../../../lib/stateView/attachStateViewHandlers');

const getTransitionHeaderFixtures = require('../../../lib/test/fixtures/getTransitionHeaderFixtures');
const getBlockFixtures = require('../../../lib/test/fixtures/getBlockFixtures');

describe('attachStateViewHandlers', () => {
  let readerMediatorMock;
  let applyStateTransition;
  let revertDapObjectsForStateTransition;
  let dropMongoDatabasesWithPrefixStub;
  let mongoDbPrefix;

  beforeEach(function beforeEach() {
    readerMediatorMock = new BlockchainReaderMediatorMock(this.sinon);
    applyStateTransition = this.sinon.stub();
    revertDapObjectsForStateTransition = this.sinon.stub();
    dropMongoDatabasesWithPrefixStub = this.sinon.stub();
    mongoDbPrefix = 'test';

    attachStateViewHandlers(
      readerMediatorMock,
      applyStateTransition,
      revertDapObjectsForStateTransition,
      dropMongoDatabasesWithPrefixStub,
      mongoDbPrefix,
    );
  });

  it('should call applyStateTransition on the state transition event', async () => {
    const [stateTransition] = getTransitionHeaderFixtures();
    const [block] = getBlockFixtures();

    await readerMediatorMock.originalEmitSerial(ReaderMediator.EVENTS.STATE_TRANSITION, {
      stateTransition,
      block,
    });

    expect(applyStateTransition).to.be.calledOnce();
    expect(applyStateTransition).to.be.calledWith(stateTransition, block);
  });

  it('should call revertDapObjectsForStateTransition on the stale state transition event', async () => {
    const [stateTransition] = getTransitionHeaderFixtures();
    const [block] = getBlockFixtures();

    await readerMediatorMock.originalEmitSerial(ReaderMediator.EVENTS.STATE_TRANSITION_STALE, {
      stateTransition,
      block,
    });

    expect(revertDapObjectsForStateTransition).to.be.calledOnce();
    expect(revertDapObjectsForStateTransition).to.be.calledWith({ stateTransition, block });
  });

  it('should call dropMongoDatabasesWithPrefix on the reset event', async () => {
    await readerMediatorMock.emit(ReaderMediator.EVENTS.RESET);

    expect(dropMongoDatabasesWithPrefixStub).to.be.calledOnce();
    expect(dropMongoDatabasesWithPrefixStub).to.be.calledWith(mongoDbPrefix);
  });
});
