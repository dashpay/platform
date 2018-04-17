const STHeadersReaderState = require('../../../lib/blockchain/reader/STHeadersReaderState');
const getBlockFixtures = require('../../../lib/test/fixtures/getBlockFixtures');

describe('STHeadersReaderState', () => {
  let blocks;
  let state;

  beforeEach(() => {
    blocks = getBlockFixtures();
    state = new STHeadersReaderState();
  });

  it('should add block and return last of them', () => {
    state.addBlock(blocks[0]);

    expect(state.getLastBlock()).to.be.equals(blocks[0]);

    state.addBlock(blocks[1]);

    expect(state.getLastBlock()).to.be.equals(blocks[1]);
  });

  it('should set blocks and return all of them', () => {
    state.setBlocks(blocks);

    expect(state.getBlocks()).to.deep.be.equals(blocks);
  });

  it('should validate blocks sequence', () => {
    state.addBlock(blocks[0]);

    expect(() => {
      state.addBlock(blocks[2]);
    }).to.be.throws('Wrong block sequence');
  });

  it('should trim blocks to limit', () => {
    const limit = 2;
    const stateWithBlocks = new STHeadersReaderState(blocks, limit);

    expect(stateWithBlocks.getBlocks()).to.be.deep.equals(blocks.slice(blocks.length - limit));
  });

  it('should clear state', () => {
    state.addBlock(blocks[0]);

    expect(state.getLastBlock()).to.be.equals(blocks[0]);

    state.clear();

    expect(state.getLastBlock()).to.be.undefined();
  });
});
