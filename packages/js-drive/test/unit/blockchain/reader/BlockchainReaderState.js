const BlockchainReaderState = require('../../../../lib/blockchain/reader/BlockchainReaderState');
const getBlockFixtures = require('../../../../lib/test/fixtures/getBlockFixtures');

describe('BlockchainReaderState', () => {
  let blocks;
  let state;

  beforeEach(() => {
    blocks = getBlockFixtures();
    state = new BlockchainReaderState();
  });

  it('should add block and return last of them', () => {
    state.addBlock(blocks[0]);

    expect(state.getLastBlock()).to.be.equal(blocks[0]);

    state.addBlock(blocks[1]);

    expect(state.getLastBlock()).to.be.equal(blocks[1]);
  });

  it('should set blocks and return all of them', () => {
    state.setBlocks(blocks);

    expect(state.getBlocks()).to.deep.be.equal(blocks);
  });

  it('should validate blocks sequence', () => {
    state.addBlock(blocks[0]);

    expect(() => {
      state.addBlock(blocks[2]);
    }).to.be.throws('Wrong block sequence');
  });

  it('should trim blocks to limit', () => {
    const limit = 2;
    const stateWithBlocks = new BlockchainReaderState(blocks, limit);

    expect(stateWithBlocks.getBlocks()).to.be.deep.equal(blocks.slice(blocks.length - limit));
  });

  it('should change blocks limit', () => {
    const limit = 4;
    const stateWithBlocks = new BlockchainReaderState(blocks, limit);
    expect(stateWithBlocks.getBlocks()).to.have.lengthOf(limit);

    const newLimit = 2;
    stateWithBlocks.setBlocksLimit(newLimit);
    expect(stateWithBlocks.getBlocks()).to.have.lengthOf(blocks.length - newLimit);
    expect(stateWithBlocks.getBlocks()).to.be.deep.equal(blocks.slice(blocks.length - newLimit));
  });

  it('should return blocks limit', () => {
    const limit = 2;
    const stateWithLimit = new BlockchainReaderState([], limit);

    expect(stateWithLimit.getBlocksLimit()).to.be.equal(limit);
  });

  it('should clear state', () => {
    state.addBlock(blocks[0]);

    expect(state.getLastBlock()).to.be.equal(blocks[0]);

    state.clear();

    expect(state.getLastBlock()).to.be.undefined();
  });
});
