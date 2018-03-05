const { expect, use } = require('chai');
const sinon = require('sinon');
const sinonChai = require('sinon-chai');
const dirtyChai = require('dirty-chai');

use(dirtyChai);
use(sinonChai);

const StateTransitionHeaderIterator = require('../../lib/blockchain/StateTransitionHeaderIterator');
const STHeadersIterationEventEmitter = require('../../lib/blockchain/STHeadersIterationEventEmitter');
const getTransitionHeaderFixtures = require('../../lib/test/fixtures/getTransitionHeaderFixtures');
const getBlockFixtures = require('../../lib/test/fixtures/getBlockFixtures');
const WrongBlocksSequenceError = require('../../lib/blockchain/WrongBlocksSequenceError');

describe('STHeadersIterationEventEmitter', () => {
  let blocks;
  let transitionHeaders;
  let emitter;
  let nextStab;
  let setStubsWithErrorOnSecondBlock;

  beforeEach(function beforeEach() {
    if (!this.sinon) {
      this.sinon = sinon.sandbox.create();
    } else {
      this.sinon.restore();
    }

    blocks = getBlockFixtures();
    transitionHeaders = getTransitionHeaderFixtures();

    let currentHeaderIndex = 0;
    let currentBlockIndex = 0;

    // Mock StateTransitionHeaderIterator
    const blockIteratorMock = {
      rpcClient: {
        getTransitionHeader: this.sinon.stub(),
      },
      getBlockHeight: this.sinon.stub().callsFake(function getBlockHeight() {
        let currentBlock = this.getCurrentBlock();
        if (!currentBlock) {
          [currentBlock] = blocks;
        }

        return currentBlock.height;
      }),
      setBlockHeight: this.sinon.stub().callsFake((height) => {
        currentBlockIndex = height - 1;
      }),
      getCurrentBlock: this.sinon.stub().callsFake(() => {
        if (currentHeaderIndex === 3) {
          currentBlockIndex += 2;
        }

        return blocks[currentBlockIndex];
      }),
      reset: this.sinon.stub(),
    };

    const stateTransitionHeaderIteratorMock = new StateTransitionHeaderIterator(blockIteratorMock);

    nextStab = this.sinon.stub(stateTransitionHeaderIteratorMock, 'next');
    nextStab.callsFake(() => {
      if (!transitionHeaders[currentHeaderIndex]) {
        return Promise.resolve({ done: true });
      }

      const currentHeader = transitionHeaders[currentHeaderIndex];

      currentHeaderIndex++;

      return Promise.resolve({ done: false, value: currentHeader });
    });
    const resetStab = this.sinon.stub(stateTransitionHeaderIteratorMock, 'reset');
    resetStab.callsFake(() => {
      currentHeaderIndex = 0;
    });

    emitter = new STHeadersIterationEventEmitter(stateTransitionHeaderIteratorMock);

    setStubsWithErrorOnSecondBlock = function setNextStubWithError() {
      blockIteratorMock.getBlockHeight.returns(blocks[1].height);

      let throwErrorOnSecondBlock = true;
      nextStab.callsFake(() => {
        if (!transitionHeaders[currentHeaderIndex]) {
          return Promise.resolve({ done: true });
        }

        if (currentHeaderIndex === blocks[0].ts.length && throwErrorOnSecondBlock) {
          currentBlockIndex = 1;
          throwErrorOnSecondBlock = false;

          throw new WrongBlocksSequenceError();
        }

        const currentHeader = transitionHeaders[currentHeaderIndex];

        currentHeaderIndex++;

        return Promise.resolve({ done: false, value: currentHeader });
      });
    };
  });

  it('should emit events while iteration over ST headers', async function it() {
    const beginHandlerStub = this.sinon.stub();
    const headerHandlerStub = this.sinon.stub();
    const blockHandlerStub = this.sinon.stub();
    const endHandlerStub = this.sinon.stub();

    emitter.on('begin', beginHandlerStub);
    emitter.on('header', headerHandlerStub);
    emitter.on('block', blockHandlerStub);
    emitter.on('end', endHandlerStub);

    await emitter.iterate();

    expect(beginHandlerStub).to.be.calledOnce();
    expect(beginHandlerStub).to.be.calledWith(blocks[0].height);

    expect(headerHandlerStub).has.callCount(transitionHeaders.length);
    transitionHeaders.forEach((header) => {
      expect(headerHandlerStub).to.be.calledWith(header);
    });

    expect(blockHandlerStub).has.callCount(2);
    expect(blockHandlerStub).to.be.calledWith(blocks[0]);
    expect(blockHandlerStub).to.be.calledWith(blocks[2]);

    expect(endHandlerStub).to.be.calledOnce();
    expect(endHandlerStub).to.be.calledWith(blocks[2].height);
  });

  it('should iterate again since stable block if blocks sequence is wrong', async function it() {
    // Stub of "next" method should throws WrongBlocksSequenceError on second block
    setStubsWithErrorOnSecondBlock();

    const headerHandlerStub = this.sinon.stub();
    const blockHandlerStub = this.sinon.stub();
    const wrongSequenceHandlerStub = this.sinon.stub();

    emitter.on('header', headerHandlerStub);
    emitter.on('block', blockHandlerStub);
    emitter.on('wrongSequence', wrongSequenceHandlerStub);

    await emitter.iterate();

    // nextStab calls: transitionHeaders.length + from from first block + error + last one empty
    expect(nextStab).has.callCount(transitionHeaders.length + blocks[0].ts.length + 2);

    // Header event emits: transitionHeaders.length + from first blocks
    expect(headerHandlerStub).has.callCount(transitionHeaders.length + blocks[0].ts.length);

    // Copy headers and duplicate headers from first block
    const transitionHeadersWithDuplicate = transitionHeaders.slice();
    // eslint-disable-next-line arrow-body-style
    const tsHeadersFromFirstTwoBlocks = blocks[0].ts.map((tsid) => {
      return transitionHeaders.find(h => h.tsid === tsid);
    });

    transitionHeadersWithDuplicate.unshift(...tsHeadersFromFirstTwoBlocks);

    transitionHeadersWithDuplicate.forEach((header) => {
      expect(headerHandlerStub).to.be.calledWith(header);
    });

    // Block event emits 3 times: first block, second block, first block again
    expect(blockHandlerStub).has.callCount(3);

    expect(blockHandlerStub).to.be.calledWith(blocks[0]);
    expect(blockHandlerStub).not.to.be.calledWith(blocks[1]);
    expect(blockHandlerStub).to.be.calledWith(blocks[2]);

    expect(wrongSequenceHandlerStub).to.be.calledOnce();
    expect(wrongSequenceHandlerStub).to.be.calledWith({
      currentBlock: blocks[1],
      previousBlock: blocks[0],
    });
  });
});
