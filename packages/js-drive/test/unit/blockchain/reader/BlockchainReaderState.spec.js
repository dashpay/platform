const BlockchainReaderState = require('../../../../lib/blockchain/reader/BlockchainReaderState');
const getBlockFixtures = require('../../../../lib/test/fixtures/getBlocksFixture');

describe('BlockchainReaderState', () => {
  let blocks;
  let state;

  beforeEach(() => {
    blocks = getBlockFixtures();
    state = new BlockchainReaderState();
  });

  it('should add a block and return it if getLastBlock() is called after', () => {
    state.addBlock(blocks[0]);

    expect(state.getLastBlock()).to.equal(blocks[0]);

    state.addBlock(blocks[1]);

    expect(state.getLastBlock()).to.equal(blocks[1]);
  });

  it('should set the blocks and return all of them', () => {
    state.setBlocks(blocks);

    expect(state.getBlocks()).to.deep.equal(blocks);
  });

  it('should validate the block sequence', () => {
    state.addBlock(blocks[0]);

    expect(() => {
      state.addBlock(blocks[2]);
    }).to.throw('Wrong block sequence');
  });

  it('should trim the blocks to a specified limit', () => {
    const limit = 2;
    const stateWithBlocks = new BlockchainReaderState(blocks, limit);

    expect(stateWithBlocks.getBlocks()).to.deep.equal(blocks.slice(blocks.length - limit));
  });

  it('should be able to change the blocks limit', () => {
    const limit = 4;
    const stateWithBlocks = new BlockchainReaderState(blocks, limit);
    expect(stateWithBlocks.getBlocks()).to.have.lengthOf(limit);

    const newLimit = 2;
    stateWithBlocks.setBlocksLimit(newLimit);
    expect(stateWithBlocks.getBlocks()).to.have.lengthOf(blocks.length - newLimit);
    expect(stateWithBlocks.getBlocks()).to.deep.equal(blocks.slice(blocks.length - newLimit));
  });

  it('should return the blocks limit', () => {
    const limit = 2;
    const stateWithLimit = new BlockchainReaderState([], limit);

    expect(stateWithLimit.getBlocksLimit()).to.equal(limit);
  });

  it('should return first synced block height');

  it('should clear an internal data if clear() is called', () => {
    state.addBlock(blocks[0]);

    expect(state.getLastBlock()).to.equal(blocks[0]);

    state.clear();

    expect(state.getLastBlock()).to.be.undefined();
  });

  it('should clear its state upon removing last block');
});
