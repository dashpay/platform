const ReaderMediator = require('../../../lib/blockchain/reader/BlockchainReaderMediator');

const BlockchainReaderMediatorMock = require('../../../lib/test/mock/BlockchainReaderMediatorMock');

const attachStateViewHandlers = require('../../../lib/stateView/attachStateViewHandlers');

const getStateTransitionsFixture = require('../../../lib/test/fixtures/getStateTransitionsFixture');
const getBlockFixtures = require('../../../lib/test/fixtures/getBlocksFixture');

describe('attachStateViewHandlers', () => {
  let readerMediatorMock;
  let applyStateTransition;
  let revertSVDocumentsForStateTransition;
  let revertSVContractsForStateTransition;
  let dropMongoDatabasesWithPrefixStub;
  let mongoDbPrefix;

  beforeEach(function beforeEach() {
    readerMediatorMock = new BlockchainReaderMediatorMock(this.sinon);
    applyStateTransition = this.sinon.stub();
    revertSVDocumentsForStateTransition = this.sinon.stub();
    revertSVContractsForStateTransition = this.sinon.stub();
    dropMongoDatabasesWithPrefixStub = this.sinon.stub();
    mongoDbPrefix = 'test';

    attachStateViewHandlers(
      readerMediatorMock,
      applyStateTransition,
      revertSVDocumentsForStateTransition,
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

    expect(applyStateTransition).to.have.been.calledOnceWith(stateTransition, block);
  });

  it('should call revertSVDocumentsForStateTransition on the orphaned state transition event', async () => {
    const [stateTransition] = getStateTransitionsFixture();
    const [block] = getBlockFixtures();

    await readerMediatorMock.originalEmitSerial(ReaderMediator.EVENTS.STATE_TRANSITION_ORPHANED, {
      stateTransition,
      block,
    });

    expect(revertSVDocumentsForStateTransition).to.have.been.calledOnceWith({
      stateTransition,
      block,
    });
  });

  it('should call revertSVContractsForStateTransition on the orphaned state transition event', async () => {
    const [stateTransition] = getStateTransitionsFixture();
    const [block] = getBlockFixtures();

    await readerMediatorMock.originalEmitSerial(ReaderMediator.EVENTS.STATE_TRANSITION_ORPHANED, {
      stateTransition,
      block,
    });

    expect(revertSVContractsForStateTransition).to.have.been.calledOnceWith({
      stateTransition,
      block,
    });
  });

  it('should call dropMongoDatabasesWithPrefix on the reset event', async () => {
    await readerMediatorMock.emit(ReaderMediator.EVENTS.RESET);

    expect(dropMongoDatabasesWithPrefixStub).to.have.been.calledOnceWith(mongoDbPrefix);
  });
});
