const BlockchainReader = require('../../../../lib/blockchain/reader/BlockchainReader');

const ArrayBlockIterator = require('../../../../lib/blockchain/blockIterator/ArrayBlockIterator');
const BlockchainReaderMediatorMock = require('../../../../lib/test/mock/BlockchainReaderMediatorMock');

const ReaderMediator = require('../../../../lib/blockchain/reader/BlockchainReaderMediator');
const RestartBlockchainReaderError = require('../../../../lib/blockchain/reader/errors/RestartBlockchainReaderError');
const IgnoreStateTransitionError = require('../../../../lib/blockchain/reader/errors/IgnoreStateTransitionError');

const getBlockFixtures = require('../../../../lib/test/fixtures/getBlocksFixture');
const getStateTransitionsFixture = require('../../../../lib/test/fixtures/getStateTransitionsFixture');

describe('BlockchainReader', () => {
  let blocks;
  let stateTransitions;
  let iteratorMock;
  let reader;
  let readerMediatorMock;
  let createStateTransitionsMock;

  beforeEach(function beforeEach() {
    blocks = getBlockFixtures();
    stateTransitions = getStateTransitionsFixture();
    iteratorMock = new ArrayBlockIterator(blocks);
    readerMediatorMock = new BlockchainReaderMediatorMock(this.sinon);
    createStateTransitionsMock = this.sinon.stub().returns([]);
    reader = new BlockchainReader(iteratorMock, readerMediatorMock, createStateTransitionsMock);
  });

  it('should set block height of the block iterator', async function it() {
    const setBlockHeightMock = this.sinon.stub(iteratorMock, 'setBlockHeight');

    await reader.read(blocks[2].height);

    expect(setBlockHeightMock).to.be.calledOnce();
    expect(setBlockHeightMock).to.be.calledWith(blocks[2].height);
  });

  it('should return zero of there are no blocks', async function it() {
    iteratorMock.blocks = [];

    this.sinon.stub(iteratorMock, 'setBlockHeight');

    const readHeight = await reader.read(1);

    expect(readHeight).to.be.equal(0);
  });

  it('should iterate over the blocks and state transitions and add iterated blocks to the state', async () => {
    const stPerBlocks = [
      [
        stateTransitions[0],
      ],
      [
        stateTransitions[1],
        stateTransitions[2],
      ],
      [
        stateTransitions[0],
        stateTransitions[1],
        stateTransitions[2],
      ],
      [],
    ];

    for (let i = 0; i < blocks.length; i++) {
      createStateTransitionsMock.onCall(i).returns(stPerBlocks[i]);
    }

    const lastBlock = blocks[blocks.length - 1];

    const readHeight = await reader.read(blocks[0].height);

    expect(readHeight).to.be.equal(lastBlock.height);

    let emitCallsCount = 0;
    for (let i = 0; i < blocks.length; i++) {
      const block = blocks[i];

      emitCallsCount++;
      expect(readerMediatorMock.emitSerial).to.be.calledWith(
        ReaderMediator.EVENTS.BLOCK_BEGIN,
        block,
      );

      expect(createStateTransitionsMock).to.be.calledWith(block);

      for await (const stateTransition of stPerBlocks[i]) {
        emitCallsCount++;
        expect(readerMediatorMock.emitSerial).to.be.calledWith(
          ReaderMediator.EVENTS.STATE_TRANSITION,
          {
            stateTransition,
            block,
          },
        );
      }

      expect(readerMediatorMock.getState().addBlock).to.be.calledWith(block);

      emitCallsCount++;
      expect(readerMediatorMock.emitSerial).to.be.calledWith(
        ReaderMediator.EVENTS.BLOCK_END,
        block,
      );
    }

    expect(readerMediatorMock.emitSerial).to.have.callCount(emitCallsCount);
  });

  it('should throw and emit error if that error happens during State Transitions iteration', async () => {
    const error = new Error();

    readerMediatorMock.emitSerial.onCall(1).throws(error);

    createStateTransitionsMock.onCall(0).returns([stateTransitions[0]]);

    let expectedError;
    try {
      await reader.read(blocks[0].height);
    } catch (e) {
      expectedError = e;
    }

    expect(expectedError).to.be.equal(error);

    expect(readerMediatorMock.emitSerial).to.have.callCount(4);

    expect(readerMediatorMock.emitSerial).to.be.calledWith(
      ReaderMediator.EVENTS.BLOCK_BEGIN,
      blocks[0],
    );

    expect(readerMediatorMock.emitSerial).to.be.calledWith(
      ReaderMediator.EVENTS.STATE_TRANSITION,
      {
        block: blocks[0],
        stateTransition: stateTransitions[0],
      },
    );

    expect(readerMediatorMock.emitSerial).to.be.calledWith(
      ReaderMediator.EVENTS.STATE_TRANSITION_ERROR,
      {
        error,
        block: blocks[0],
        stateTransition: stateTransitions[0],
      },
    );

    expect(readerMediatorMock.emitSerial).to.be.not.calledWith(
      ReaderMediator.EVENTS.BLOCK_ERROR,
      {
        error,
        block: blocks[0],
        stateTransitions: stateTransitions[0],
      },
    );
  });

  it('should throw error from State Transition error handler', async () => {
    const error = new Error();
    const errorFromErrorHandler = new Error();

    readerMediatorMock.emitSerial.onCall(1).throws(error);
    readerMediatorMock.emitSerial.onCall(2).throws(errorFromErrorHandler);

    createStateTransitionsMock.onCall(0).returns([stateTransitions[0]]);

    let expectedError;
    try {
      await reader.read(blocks[0].height);
    } catch (e) {
      expectedError = e;
    }

    expect(expectedError).to.be.equal(errorFromErrorHandler);

    expect(readerMediatorMock.emitSerial).to.have.callCount(4);

    expect(readerMediatorMock.emitSerial).to.be.calledWith(
      ReaderMediator.EVENTS.BLOCK_BEGIN,
      blocks[0],
    );

    expect(readerMediatorMock.emitSerial).to.be.calledWith(
      ReaderMediator.EVENTS.STATE_TRANSITION,
      {
        block: blocks[0],
        stateTransition: stateTransitions[0],
      },
    );

    expect(readerMediatorMock.emitSerial).to.be.calledWith(
      ReaderMediator.EVENTS.STATE_TRANSITION_ERROR,
      {
        error,
        block: blocks[0],
        stateTransition: stateTransitions[0],
      },
    );

    expect(readerMediatorMock.emitSerial).to.be.not.calledWith(
      ReaderMediator.EVENTS.BLOCK_ERROR,
      {
        error,
        block: blocks[0],
        stateTransitions: stateTransitions[0],
      },
    );
  });

  it('should continue ST iteration if IgnoreStateTransitionError thrown'
    + ' from State Transition error handler', async () => {
    const stPerBlocks = [
      [
        stateTransitions[0],
      ],
      [
        stateTransitions[1],
        stateTransitions[2],
      ],
      [
        stateTransitions[0],
        stateTransitions[1],
        stateTransitions[2],
      ],
      [],
    ];

    const error = new Error();
    const ignoreError = new IgnoreStateTransitionError();

    readerMediatorMock.emitSerial.onCall(1).throws(error);
    readerMediatorMock.emitSerial.onCall(2).throws(ignoreError);

    for (let i = 0; i < blocks.length; i++) {
      createStateTransitionsMock.onCall(i).returns(stPerBlocks[i]);
    }

    const readHeight = await reader.read(blocks[0].height);

    expect(readHeight).to.be.equal(blocks[blocks.length - 1].height);

    expect(readerMediatorMock.emitSerial).to.be.calledWith(
      ReaderMediator.EVENTS.STATE_TRANSITION_ERROR,
      {
        error,
        block: blocks[0],
        stateTransition: stateTransitions[0],
      },
    );

    for (let i = 0; i < blocks.length; i++) {
      const block = blocks[i];

      for await (const stateTransition of stPerBlocks[i]) {
        expect(readerMediatorMock.emitSerial).to.be.calledWith(
          ReaderMediator.EVENTS.STATE_TRANSITION,
          {
            stateTransition,
            block,
          },
        );
      }
    }
  });

  it('should throw and emit error if that error happens during block iteration', async () => {
    const error = new Error();

    readerMediatorMock.emitSerial.onCall(0).throws(error);

    let expectedError;
    try {
      await reader.read(blocks[0].height);
    } catch (e) {
      expectedError = e;
    }

    expect(expectedError).to.be.equal(error);

    expect(readerMediatorMock.emitSerial).to.be.calledTwice();

    expect(readerMediatorMock.emitSerial).to.be.calledWith(
      ReaderMediator.EVENTS.BLOCK_BEGIN,
      blocks[0],
    );

    expect(readerMediatorMock.emitSerial).to.be.calledWith(
      ReaderMediator.EVENTS.BLOCK_ERROR,
      {
        error,
        block: blocks[0],
        stateTransition: null,
      },
    );

    expect(readerMediatorMock.emitSerial).to.be.not.calledWith(
      ReaderMediator.EVENTS.BLOCK_BEGIN,
      blocks[1],
    );
  });

  it('should restart reading if RestartBlockchainReaderError thrown from the block error event handlers', async () => {
    const error = new Error();
    const restartError = new RestartBlockchainReaderError(blocks[3].height);

    readerMediatorMock.emitSerial.onCall(0).throws(error);

    readerMediatorMock.emitSerial.onCall(1).throws(restartError);

    const readHeight = await reader.read(blocks[0].height);

    expect(readHeight).to.be.equal(blocks[3].height);

    expect(readerMediatorMock.emitSerial).to.be.calledWith(
      ReaderMediator.EVENTS.BLOCK_ERROR,
      {
        error,
        block: blocks[0],
        stateTransition: null,
      },
    );
  });

  it('should throw error from block error handler', async () => {
    const error = new Error();
    const errorFromErrorHandler = new Error();

    readerMediatorMock.emitSerial.onCall(0).throws(error);

    readerMediatorMock.emitSerial.onCall(1).throws(errorFromErrorHandler);

    let expectedError;
    try {
      await reader.read(blocks[0].height);
    } catch (e) {
      expectedError = e;
    }

    expect(expectedError).to.be.equal(errorFromErrorHandler);

    expect(readerMediatorMock.emitSerial).to.be.calledWith(
      ReaderMediator.EVENTS.BLOCK_ERROR,
      {
        error,
        block: blocks[0],
        stateTransition: null,
      },
    );

    expect(readerMediatorMock.emitSerial).to.be.not.calledWith(
      ReaderMediator.EVENTS.BLOCK_BEGIN,
      blocks[1],
    );
  });
});
