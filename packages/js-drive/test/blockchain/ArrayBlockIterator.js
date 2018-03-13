const { expect, use } = require('chai');
const sinon = require('sinon');
const sinonChai = require('sinon-chai');
const dirtyChai = require('dirty-chai');

use(sinonChai);
use(dirtyChai);

const getBlockFixtures = require('../../lib/test/fixtures/getBlockFixtures');
const ArrayBlockIterator = require('../../lib/blockchain/ArrayBlockIterator');

describe('ArrayBlockIterator', () => {
  let blocks;
  let blockIterator;

  beforeEach(function beforeEach() {
    if (!this.sinon) {
      this.sinon = sinon.sandbox.create();
    } else {
      this.sinon.restore();
    }

    blocks = getBlockFixtures();
    blockIterator = new ArrayBlockIterator(blocks);
  });

  it('should iterate over blocks', async () => {
    const obtainedBlocks = [];

    let done;
    let block;

    // eslint-disable-next-line no-cond-assign
    while ({ done, value: block } = await blockIterator.next()) {
      if (done) {
        break;
      }

      obtainedBlocks.push(block);
    }

    expect(obtainedBlocks).to.be.deep.equal(blocks);
  });

  it('should iterate from begging when "reset" method is called', async () => {
    const { value: firstBlock } = await blockIterator.next();

    blockIterator.reset();

    const { value: secondBlock } = await blockIterator.next();

    expect(firstBlock).to.be.equal(secondBlock);
  });

  it("should emit 'block' event", async function it() {
    const blockHandlerStub = this.sinon.stub();

    blockIterator.on('block', blockHandlerStub);

    const { value: firstBlock } = await blockIterator.next();

    expect(blockHandlerStub).to.be.calledOnce();
    expect(blockHandlerStub).to.be.calledWith(firstBlock);

    const { value: secondBlock } = await blockIterator.next();

    expect(blockHandlerStub).to.be.calledTwice();
    expect(blockHandlerStub).to.be.calledWith(secondBlock);
  });
});
