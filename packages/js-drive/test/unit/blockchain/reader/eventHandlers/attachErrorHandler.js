const BlockchainReaderMediatorMock = require('../../../../../lib/test/mock/BlockchainReaderMediatorMock');

const ReaderMediator = require('../../../../../lib/blockchain/reader/BlockchainReaderMediator');
const RestartBlockchainReaderError = require('../../../../../lib/blockchain/reader/errors/RestartBlockchainReaderError');
const IgnoreStateTransitionError = require('../../../../../lib/blockchain/reader/errors/IgnoreStateTransitionError');

const getBlockFixtures = require('../../../../../lib/test/fixtures/getBlockFixtures');
const getTransitionHeaderFixtures = require('../../../../../lib/test/fixtures/getTransitionHeaderFixtures');

const attachErrorHandler = require('../../../../../lib/blockchain/reader/eventHandlers/attachErrorHandler');

describe('attachErrorHandler', () => {
  let readerMediatorMock;
  let blocks;
  let stateTransitions;

  beforeEach(function beforeEach() {
    readerMediatorMock = new BlockchainReaderMediatorMock(this.sinon);

    blocks = getBlockFixtures();
    stateTransitions = getTransitionHeaderFixtures();
  });

  describe('Block error handler', () => {
    it('should do nothing if skipBlockWithErrors is disabled', async () => {
      attachErrorHandler(
        readerMediatorMock,
        {
          skipBlockWithErrors: false,
        },
      );

      await readerMediatorMock.originalEmitSerial(
        ReaderMediator.EVENTS.BLOCK_ERROR,
        { block: blocks[0] },
      );

      expect(readerMediatorMock.emitSerial).to.be.not.called();
    });

    it('should restart reader from the next firstBlock if skipBlockWithErrors is enabled', async () => {
      attachErrorHandler(
        readerMediatorMock,
        {
          skipBlockWithErrors: true,
        },
      );

      let expectedError;
      try {
        await readerMediatorMock.originalEmitSerial(
          ReaderMediator.EVENTS.BLOCK_ERROR,
          { block: blocks[0] },
        );
      } catch (e) {
        expectedError = e;
      }

      expect(expectedError).to.be.instanceOf(RestartBlockchainReaderError);
      expect(expectedError.getHeight()).to.be.equal(blocks[1].height);

      expect(readerMediatorMock.emitSerial).to.be.calledOnceWith(
        ReaderMediator.EVENTS.BLOCK_SKIP,
        blocks[0],
      );
    });

    it('should not restart reader if the current firstBlock is the last one', async () => {
      attachErrorHandler(
        readerMediatorMock,
        {
          skipBlockWithErrors: true,
        },
      );

      await readerMediatorMock.originalEmitSerial(
        ReaderMediator.EVENTS.BLOCK_ERROR,
        { block: blocks[3] },
      );

      expect(readerMediatorMock.emitSerial).to.be.calledOnceWith(
        ReaderMediator.EVENTS.BLOCK_SKIP,
        blocks[3],
      );
    });
  });

  describe('State Transition error handler', () => {
    it('should do nothing if skipStateTransitionWithErrors is disabled', async () => {
      attachErrorHandler(
        readerMediatorMock,
        {
          skipStateTransitionWithErrors: false,
        },
      );

      await readerMediatorMock.originalEmitSerial(
        ReaderMediator.EVENTS.STATE_TRANSITION_ERROR,
        {
          block: blocks[0],
          stateTransition: stateTransitions[0],
        },
      );

      expect(readerMediatorMock.emitSerial).to.be.not.called();
    });

    it('should skip State Transition if skipStateTransitionWithErrors is enabled', async () => {
      const [stateTransition] = stateTransitions;

      attachErrorHandler(
        readerMediatorMock,
        {
          skipStateTransitionWithErrors: true,
        },
      );

      let expectedError;
      try {
        await readerMediatorMock.originalEmitSerial(
          ReaderMediator.EVENTS.STATE_TRANSITION_ERROR,
          {
            block: blocks[0],
            stateTransition,
          },
        );
      } catch (e) {
        expectedError = e;
      }

      expect(expectedError).to.be.instanceOf(IgnoreStateTransitionError);

      expect(readerMediatorMock.emitSerial).to.be.calledOnceWith(
        ReaderMediator.EVENTS.STATE_TRANSITION_SKIP,
        {
          block: blocks[0],
          stateTransition,
        },
      );
    });
  });
});
