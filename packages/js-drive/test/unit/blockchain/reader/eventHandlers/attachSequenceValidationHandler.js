const BlockchainReaderMediatorMock = require('../../../../../lib/test/mock/BlockchainReaderMediatorMock');

const ReaderMediator = require('../../../../../lib/blockchain/reader/BlockchainReaderMediator');
const RestartBlockchainReaderError = require('../../../../../lib/blockchain/reader/errors/RestartBlockchainReaderError');

const WrongSequenceError = require('../../../../../lib/blockchain/reader/eventHandlers/errors/WrongSequenceError');
const NotAbleToValidateSequenceError = require('../../../../../lib/blockchain/reader/eventHandlers/errors/NotAbleToValidateSequenceError');

const attachSequenceValidationHandler = require('../../../../../lib/blockchain/reader/eventHandlers/attachSequenceValidationHandler');

const getBlockFixtures = require('../../../../../lib/test/fixtures/getBlockFixtures');
const getTransitionHeaderFixtures = require('../../../../../lib/test/fixtures/getTransitionHeaderFixtures');

describe('attachSequenceValidationHandler', () => {
  let readerMediatorMock;
  let createStateTransitionsMock;
  let blocks;

  beforeEach(function beforeEach() {
    readerMediatorMock = new BlockchainReaderMediatorMock(this.sinon);

    createStateTransitionsMock = this.sinon.stub();

    attachSequenceValidationHandler(
      readerMediatorMock,
      createStateTransitionsMock,
    );

    blocks = getBlockFixtures();
  });

  describe('sequence validation', () => {
    it('should not able to validate sequence if the last synced block is not present'
      + 'and current block is not initial', async () => {
      const [currentBlock] = blocks;
      currentBlock.height = 5;

      readerMediatorMock.getInitialBlockHeight.returns(1);

      try {
        await readerMediatorMock.originalEmitSerial(
          ReaderMediator.EVENTS.BLOCK_BEGIN,
          currentBlock,
        );
      } catch (e) {
        if (e instanceof NotAbleToValidateSequenceError) {
          expect(readerMediatorMock.getState().getLastBlock).to.be.calledOnce();
          expect(readerMediatorMock.getState().getFirstBlockHeight).to.be.calledOnce();

          expect(readerMediatorMock.getInitialBlockHeight).to.be.calledOnce();

          return;
        }

        throw e;
      }

      expect.fail('sequence is correct');
    });

    it('should detect sequence as correct if this last synced block is not present'
      + 'and current block is initial', async () => {
      const [currentBlock] = blocks;
      currentBlock.height = 1;

      readerMediatorMock.getInitialBlockHeight.returns(1);

      await readerMediatorMock.originalEmitSerial(ReaderMediator.EVENTS.BLOCK_BEGIN, currentBlock);

      expect(readerMediatorMock.getState().getLastBlock).to.be.calledOnce();
      expect(readerMediatorMock.getState().getFirstBlockHeight).to.be.calledOnce();

      expect(readerMediatorMock.getInitialBlockHeight).to.be.calledOnce();
    });

    it('should detect sequence as correct if current block higher than the last synced block', async () => {
      const [currentBlock, lastSyncedBlock] = blocks;
      currentBlock.height = 2;
      currentBlock.previousblockhash = 'hash';

      lastSyncedBlock.height = 1;
      lastSyncedBlock.hash = 'hash';

      readerMediatorMock.getState().getLastBlock.returns(lastSyncedBlock);

      await readerMediatorMock.originalEmitSerial(
        ReaderMediator.EVENTS.BLOCK_BEGIN,
        currentBlock,
      );

      expect(readerMediatorMock.getState().getLastBlock).to.be.calledOnce();
      expect(readerMediatorMock.getState().getFirstBlockHeight).to.be.calledOnce();

      expect(readerMediatorMock.getInitialBlockHeight).to.be.not.called();
    });

    it('should not able to validate sequence if current block'
      + ' lower or equal then the first synced block', async () => {
      const [currentBlock, lastSyncedBlock] = blocks;
      currentBlock.height = 1;
      lastSyncedBlock.height = 5;

      readerMediatorMock.getState().getLastBlock.returns(lastSyncedBlock);
      readerMediatorMock.getState().getFirstBlockHeight.returns(1);

      try {
        await readerMediatorMock.originalEmitSerial(
          ReaderMediator.EVENTS.BLOCK_BEGIN,
          currentBlock,
        );
      } catch (e) {
        if (e instanceof NotAbleToValidateSequenceError) {
          expect(readerMediatorMock.getState().getLastBlock).to.be.calledOnce();
          expect(readerMediatorMock.getState().getFirstBlockHeight).to.be.calledOnce();

          expect(readerMediatorMock.getInitialBlockHeight).to.be.not.called();

          return;
        }

        throw e;
      }

      expect.fail('sequence is correct');
    });

    it('should detect sequence as correct if current block higher then the first synced block', async () => {
      const [currentBlock, lastSyncedBlock] = blocks;
      currentBlock.height = 2;
      lastSyncedBlock.height = 5;

      readerMediatorMock.getState().getLastBlock.returns(lastSyncedBlock);
      readerMediatorMock.getState().getFirstBlockHeight.returns(1);

      await readerMediatorMock.originalEmitSerial(
        ReaderMediator.EVENTS.BLOCK_BEGIN,
        currentBlock,
      );

      expect(readerMediatorMock.getState().getLastBlock).to.be.calledOnce();
      expect(readerMediatorMock.getState().getFirstBlockHeight).to.be.calledOnce();

      expect(readerMediatorMock.getInitialBlockHeight).to.be.not.called();
    });

    it('should detect sequence as wrong if the last synced block hash is not equal'
      + ' to the current block\'s "previousblockhash"', async () => {
      const [currentBlock, lastSyncedBlock] = blocks;
      currentBlock.height = 2;
      currentBlock.previousblockhash = 'differentHash';

      lastSyncedBlock.height = 1;
      lastSyncedBlock.hash = 'hash';

      readerMediatorMock.getState().getLastBlock.returns(lastSyncedBlock);

      try {
        await readerMediatorMock.originalEmitSerial(
          ReaderMediator.EVENTS.BLOCK_BEGIN,
          currentBlock,
        );
      } catch (e) {
        if (e instanceof WrongSequenceError) {
          expect(readerMediatorMock.getState().getLastBlock).to.be.calledOnce();
          expect(readerMediatorMock.getState().getFirstBlockHeight).to.be.calledOnce();

          expect(readerMediatorMock.getInitialBlockHeight).to.be.not.called();

          return;
        }

        throw e;
      }

      expect.fail('sequence is correct');
    });
  });

  describe('validation errors handler', () => {
    it('should do nothing if no validation errors', async () => {
      const error = new Error();

      await readerMediatorMock.originalEmitSerial(ReaderMediator.EVENTS.BLOCK_ERROR, error);
    });

    it('should restart reader from initial block if not able not validate sequence', async () => {
      const initialBlockHeight = 1;

      readerMediatorMock.getInitialBlockHeight.returns(initialBlockHeight);

      try {
        const error = new NotAbleToValidateSequenceError();

        await readerMediatorMock.originalEmitSerial(
          ReaderMediator.EVENTS.BLOCK_ERROR,
          {
            error,
            block: blocks[0],
          },
        );
      } catch (e) {
        if (e instanceof RestartBlockchainReaderError) {
          expect(readerMediatorMock.reset).to.be.calledOnce();

          expect(readerMediatorMock.getInitialBlockHeight).to.be.calledOnce();

          expect(e.getHeight()).to.be.equal(initialBlockHeight);

          return;
        }

        throw e;
      }

      expect.fail('reader was not restarted');
    });

    it('should restart reader from the next block after the last synced if the current block height'
      + ' greater than one', async () => {
      const [currentBlock, lastSyncedBlock] = blocks;
      currentBlock.height = 3;
      lastSyncedBlock.height = 1;

      readerMediatorMock.getState().getLastBlock.returns(lastSyncedBlock);

      try {
        const error = new WrongSequenceError();

        await readerMediatorMock.originalEmitSerial(ReaderMediator.EVENTS.BLOCK_ERROR, {
          block: currentBlock,
          error,
        });
      } catch (e) {
        if (e instanceof RestartBlockchainReaderError) {
          expect(readerMediatorMock.getState().removeLastBlock).to.be.not.called();

          expect(readerMediatorMock.emitSerial).to.be.not.called();

          expect(e.getHeight()).to.be.equal(lastSyncedBlock.height + 1);

          expect(readerMediatorMock.reset).to.be.not.called();

          return;
        }

        throw e;
      }

      expect.fail('reader was not restarted');
    });

    it('should restart reader form the previous block if the last synced block height is lower than'
      + ' the current block height', async () => {
      const [currentBlock, lastSyncedBlock] = blocks;
      currentBlock.height = 2;
      lastSyncedBlock.height = 1;

      const stateTransitions = getTransitionHeaderFixtures();

      readerMediatorMock.getState().getLastBlock.returns(lastSyncedBlock);

      createStateTransitionsMock.returns(stateTransitions);

      try {
        const error = new WrongSequenceError();

        await readerMediatorMock.originalEmitSerial(ReaderMediator.EVENTS.BLOCK_ERROR, {
          block: currentBlock,
          error,
        });
      } catch (e) {
        if (e instanceof RestartBlockchainReaderError) {
          expect(readerMediatorMock.getState().removeLastBlock).to.be.calledOnce();

          expect(readerMediatorMock.emitSerial).to.have.callCount(stateTransitions.length + 1);

          expect(readerMediatorMock.emitSerial).to.be.calledWith(
            ReaderMediator.EVENTS.BLOCK_STALE,
            lastSyncedBlock,
          );

          for (const stateTransition of stateTransitions) {
            expect(readerMediatorMock.emitSerial).to.be.calledWith(
              ReaderMediator.EVENTS.STATE_TRANSITION_STALE,
              {
                stateTransition,
                block: lastSyncedBlock,
              },
            );
          }

          expect(e.getHeight()).to.be.equal(currentBlock.height - 1);

          expect(readerMediatorMock.reset).to.be.not.called();

          return;
        }

        throw e;
      }

      expect.fail('reader was not restarted');
    });

    it('should restart reader from the current block if the last synced block height is greater or equal to'
      + ' the current block height', async () => {
      const [currentBlock, lastSyncedBlock] = blocks;
      currentBlock.height = 2;
      lastSyncedBlock.height = 3;

      const stateTransitions = getTransitionHeaderFixtures();

      readerMediatorMock.getState().getLastBlock.returns(lastSyncedBlock);

      createStateTransitionsMock.returns(stateTransitions);

      try {
        const error = new WrongSequenceError();

        await readerMediatorMock.originalEmitSerial(ReaderMediator.EVENTS.BLOCK_ERROR, {
          block: currentBlock,
          error,
        });
      } catch (e) {
        if (e instanceof RestartBlockchainReaderError) {
          expect(readerMediatorMock.getState().removeLastBlock).to.be.calledOnce();

          expect(readerMediatorMock.emitSerial).to.have.callCount(stateTransitions.length + 1);

          expect(readerMediatorMock.emitSerial).to.be.calledWith(
            ReaderMediator.EVENTS.BLOCK_STALE,
            lastSyncedBlock,
          );

          for (const stateTransition of stateTransitions) {
            expect(readerMediatorMock.emitSerial).to.be.calledWith(
              ReaderMediator.EVENTS.STATE_TRANSITION_STALE,
              {
                stateTransition,
                block: lastSyncedBlock,
              },
            );
          }

          expect(e.getHeight()).to.be.equal(currentBlock.height);

          expect(readerMediatorMock.reset).to.be.not.called();

          return;
        }

        throw e;
      }

      expect.fail('reader was not restarted');
    });
  });
});
