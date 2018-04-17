const SyncState = require('../../../lib/sync/state/SyncState');
const getBlockFixtures = require('../../../lib/test/fixtures/getBlockFixtures');

describe('SyncState', () => {
  let blocks;
  let state;

  beforeEach(() => {
    blocks = getBlockFixtures();
    state = new SyncState([], new Date());
  });

  it('should set and return blocks', () => {
    state.setBlocks(blocks);

    expect(state.getBlocks()).to.be.equals(blocks);
  });

  it('should set and return last sync date', () => {
    const date = new Date();

    state.setLastSyncAt(date);

    expect(state.getLastSyncAt()).to.be.equals(date);
  });

  it('should accept block and last sync date in constructor', () => {
    const passedBlocks = [blocks[0]];
    const passedDate = new Date();
    state = new SyncState(passedBlocks, passedDate);

    expect(state.getBlocks()).to.be.equals(passedBlocks);
    expect(state.getLastSyncAt()).to.be.equals(passedDate);
  });

  it('should return last block', () => {
    state.setBlocks(blocks);

    expect(state.getLastBlock()).to.be.equals(blocks[blocks.length - 1]);
  });

  it('should compare state data with another sync state', () => {
    const anotherState = new SyncState(blocks, new Date());
    const oneMoreState = new SyncState(
      state.getBlocks(),
      new Date(state.getLastSyncAt()),
    );

    expect(state.isEqual(anotherState)).to.be.false();
    expect(state.isEqual(oneMoreState)).to.be.true();
  });

  it('should return plain data', () => {
    state.setBlocks(blocks);

    expect(state.toJSON()).to.be.deep.equals({
      blocks: state.getBlocks(),
      lastSyncAt: state.getLastSyncAt(),
    });
  });
});
