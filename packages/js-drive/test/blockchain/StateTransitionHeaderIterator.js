const { expect, use } = require('chai');
const sinon = require('sinon');
const sinonChai = require('sinon-chai');

use(sinonChai);

const StateTransitionHeaderIterator = require('../../lib/blockchain/StateTransitionHeaderIterator');
const getTransitionHeaderFixtures = require('../../lib/test/fixtures/getTransitionHeaderFixtures');
const getBlockFixtures = require('../../lib/test/fixtures/getBlockFixtures');

describe('StateTransitionHeaderIterator', () => {
  let blocks;
  let transitionHeaders;
  let blockIteratorMock;
  let getTransitionHeaderSpy;
  let nextSpy;

  beforeEach(function beforeEach() {
    if (!this.sinon) {
      this.sinon = sinon.sandbox.create();
    } else {
      this.sinon.restore();
    }

    blocks = getBlockFixtures();
    transitionHeaders = getTransitionHeaderFixtures();

    let currentBlockIndex = 0;
    blockIteratorMock = {
      rpcClient: {
        getTransitionHeader(tsid, callback) {
          callback(null, { result: transitionHeaders.find(h => h.getHash() === tsid) });
        },
      },
      getBlockHeight() {
        return blocks[currentBlockIndex].height;
      },
      async next() {
        if (!blocks[currentBlockIndex]) {
          return Promise.resolve({ done: true });
        }

        const currentBlock = blocks[currentBlockIndex];

        currentBlockIndex++;

        return Promise.resolve({ done: false, value: currentBlock });
      },
      reset() {
        currentBlockIndex = 0;
      },
    };

    getTransitionHeaderSpy = this.sinon.spy(blockIteratorMock.rpcClient, 'getTransitionHeader');
    nextSpy = this.sinon.spy(blockIteratorMock, 'next');
  });

  it('should iterate over State Transitions from BlockIterator', async () => {
    const obtainedTransitionHeaders = [];

    const stateTransitionHeaderIterator = new StateTransitionHeaderIterator(blockIteratorMock);

    let done;
    let header;

    // eslint-disable-next-line no-cond-assign
    while ({ done, value: header } = await stateTransitionHeaderIterator.next()) {
      if (done) {
        break;
      }

      obtainedTransitionHeaders.push(header);
    }

    expect(getTransitionHeaderSpy).has.callCount(transitionHeaders.length);
    expect(nextSpy).has.callCount(blocks.length + 1);

    const obtainedHeaderHashes = obtainedTransitionHeaders.map(h => h.getHash());
    const transitionHeaderHashes = transitionHeaders.map(h => h.getHash());

    expect(obtainedHeaderHashes).to.be.deep.equal(transitionHeaderHashes);
  });

  it('should iterate from begging when "reset" method is called', async () => {
    const stateTransitionHeaderIterator = new StateTransitionHeaderIterator(blockIteratorMock);

    const { value: firstHeader } = await stateTransitionHeaderIterator.next();

    stateTransitionHeaderIterator.reset();

    const { value: secondHeader } = await stateTransitionHeaderIterator.next();

    expect(firstHeader.getHash()).to.be.equal(secondHeader.getHash());
  });
});
