const getBlockFixtures = require('../../../../lib/test/fixtures/getBlocksFixture');
const ArrayBlockIterator = require('../../../../lib/blockchain/blockIterator/ArrayBlockIterator');
const InvalidBlockHeightError = require('../../../../lib/blockchain/blockIterator/InvalidBlockHeightError');

describe('ArrayBlockIterator', () => {
  let blocks;
  let blockIterator;

  beforeEach(() => {
    blocks = getBlockFixtures();
    blockIterator = new ArrayBlockIterator(blocks);
  });

  it('should iterate over blocks', async () => {
    const obtainedBlocks = [];

    for await (const block of blockIterator) {
      obtainedBlocks.push(block);
    }

    expect(obtainedBlocks).to.deep.equal(blocks);
  });

  it('should iterate from beginning when "reset" method is called', async () => {
    const { value: firstBlock } = await blockIterator.next();

    blockIterator.reset();

    const { value: secondBlock } = await blockIterator.next();

    expect(firstBlock).to.equal(secondBlock);
  });

  describe('setBlockHeight', () => {
    it('should set block height', async () => {
      const { value: firstBlock } = await blockIterator.next();

      blockIterator.setBlockHeight(blocks[2].height);

      const { value: thirdBlock } = await blockIterator.next();

      expect(firstBlock).to.equal(blocks[0]);
      expect(thirdBlock).to.equal(blocks[2]);
    });
    it('should throw an error if there is no block with specified height', () => {
      expect(() => blockIterator.setBlockHeight(999)).to.throw(InvalidBlockHeightError);
    });
  });

  describe('getBlockHeight', () => {
    it('should return block height', async () => {
      const firstBlockHeight = blockIterator.getBlockHeight();

      await blockIterator.next();

      const secondBlockHeight = blockIterator.getBlockHeight();

      expect(firstBlockHeight).to.equal(blocks[0].height);
      expect(secondBlockHeight).to.equal(blocks[1].height);
    });

    it('should throw an error if there are no blocks', async () => {
      blockIterator = new ArrayBlockIterator([]);
      expect(() => blockIterator.getBlockHeight()).to.throw(InvalidBlockHeightError);
    });
  });
});
