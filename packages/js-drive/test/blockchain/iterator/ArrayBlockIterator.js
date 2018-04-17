const getBlockFixtures = require('../../../lib/test/fixtures/getBlockFixtures');
const ArrayBlockIterator = require('../../../lib/blockchain/iterator/ArrayBlockIterator');

describe('ArrayBlockIterator', () => {
  let blocks;
  let blockIterator;

  beforeEach(() => {
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
