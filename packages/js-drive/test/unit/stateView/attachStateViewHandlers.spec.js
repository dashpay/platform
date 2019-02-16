const ReaderMediator = require('../../../lib/blockchain/reader/BlockchainReaderMediator');

const BlockchainReaderMediatorMock = require('../../../lib/test/mock/BlockchainReaderMediatorMock');

const attachStateViewHandlers = require('../../../lib/stateView/attachStateViewHandlers');

const getStateTransitionsFixture = require('../../../lib/test/fixtures/getStateTransitionsFixture');
const getBlockFixtures = require('../../../lib/test/fixtures/getBlocksFixture');

describe('attachStateViewHandlers', () => {
  let readerMediatorMock;
  let applyStateTransition;
  let revertSVObjectsForStateTransition;
  let revertSVContractsForStateTransition;
  let dropMongoDatabasesWithPrefixStub;
  let mongoDbPrefix;

  beforeEach(function beforeEach() {
    readerMediatorMock = new BlockchainReaderMediatorMock(this.sinon);
    applyStateTransition = this.sinon.stub();
    revertSVObjectsForStateTransition = this.sinon.stub();
    revertSVContractsForStateTransition = this.sinon.stub();
    dropMongoDatabasesWithPrefixStub = this.sinon.stub();
    mongoDbPrefix = 'test';

    attachStateViewHandlers(
      readerMediatorMock,
      applyStateTransition,
      revertSVObjectsForStateTransition,
      revertSVContractsForStateTransition,
      dropMongoDatabasesWithPrefixStub,
      mongoDbPrefix,
    );
  });

  it('should call applyStateTransition on the state transition event', async () => {
    const [stateTransition] = getStateTransitionsFixture();
    const [block] = getBlockFixtures();

    await readerMediatorMock.originalEmitSerial(ReaderMediator.EVENTS.STATE_TRANSITION, {
      stateTransition,
      block,
    });

    expect(applyStateTransition).to.be.calledOnce();
    expect(applyStateTransition).to.be.calledWith(stateTransition, block);
  });

  it('should call revertSVObjectsForStateTransition on the orphaned state transition event', async () => {
    const [stateTransition] = getStateTransitionsFixture();
    const [block] = getBlockFixtures();

    await readerMediatorMock.originalEmitSerial(ReaderMediator.EVENTS.STATE_TRANSITION_ORPHANED, {
      stateTransition,
      block,
    });

    expect(revertSVObjectsForStateTransition).to.be.calledOnce();
    expect(revertSVObjectsForStateTransition).to.be.calledWith({ stateTransition, block });
  });

  it('should call revertSVContractsForStateTransition on the orphaned state transition event', async () => {
    const [stateTransition] = getStateTransitionsFixture();
    const [block] = getBlockFixtures();

    await readerMediatorMock.originalEmitSerial(ReaderMediator.EVENTS.STATE_TRANSITION_ORPHANED, {
      stateTransition,
      block,
    });

    expect(revertSVContractsForStateTransition).to.be.calledOnce();
    expect(revertSVContractsForStateTransition).to.be.calledWith({ stateTransition, block });
  });

  it('should call dropMongoDatabasesWithPrefix on the reset event', async () => {
    await readerMediatorMock.emit(ReaderMediator.EVENTS.RESET);

    expect(dropMongoDatabasesWithPrefixStub).to.be.calledOnce();
    expect(dropMongoDatabasesWithPrefixStub).to.be.calledWith(mongoDbPrefix);
  });
});
