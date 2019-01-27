const SyncInfo = require('../../../../lib/sync/info/SyncInfo');
const getBlockFixtures = require('../../../../lib/test/fixtures/getBlocksFixture');

describe('SyncInfo', () => {
  const blocks = getBlockFixtures();

  it('should serialize SyncSatus', () => {
    const lastSyncedBlock = blocks[0];
    const lastChainBlock = blocks[3];
    const lastSyncAt = new Date();
    const lastInitialSyncAt = new Date();
    const isBlockChainSynced = false;
    const syncInfo = new SyncInfo(
      lastSyncedBlock.height,
      lastSyncedBlock.hash,
      lastSyncAt,
      lastInitialSyncAt,
      lastChainBlock.height,
      lastChainBlock.hash,
      isBlockChainSynced,
    );
    expect(syncInfo.toJSON()).to.be.deep.equal({
      lastSyncedBlockHeight: lastSyncedBlock.height,
      lastSyncedBlockHash: lastSyncedBlock.hash,
      lastSyncAt,
      lastInitialSyncAt,
      lastChainBlockHeight: lastChainBlock.height,
      lastChainBlockHash: lastChainBlock.hash,
      status: SyncInfo.STATUSES.SYNCING,
    });
  });

  describe('getStatus', () => {
    it('should be INITIAL_SYNC if has not lastSyncAt', async () => {
      const lastSyncedBlock = blocks[0];
      const lastChainBlock = blocks[3];
      const lastSyncAt = null;
      const lastInitialSyncAt = null;
      const isBlockChainSynced = false;

      const syncInfo = new SyncInfo(
        lastSyncedBlock.height,
        lastSyncedBlock.hash,
        lastSyncAt,
        lastInitialSyncAt,
        lastChainBlock.height,
        lastChainBlock.hash,
        isBlockChainSynced,
      );
      expect(syncInfo.getStatus()).to.be.equal(SyncInfo.STATUSES.INITIAL_SYNC);
    });

    it('should be INITIAL_SYNC if has not lastInitialSyncAt', async () => {
      const lastSyncedBlock = blocks[0];
      const lastChainBlock = blocks[3];
      const lastSyncAt = null;
      const lastInitialSyncAt = null;
      const isBlockChainSynced = false;

      const syncInfo = new SyncInfo(
        lastSyncedBlock.height,
        lastSyncedBlock.hash,
        lastSyncAt,
        lastInitialSyncAt,
        lastChainBlock.height,
        lastChainBlock.hash,
        isBlockChainSynced,
      );
      expect(syncInfo.getStatus()).to.be.equal(SyncInfo.STATUSES.INITIAL_SYNC);
    });

    it('should be SYNCING if has lastSyncAt and isBlockchainSynced is false', async () => {
      const lastSyncedBlock = blocks[0];
      const lastChainBlock = blocks[3];
      const lastSyncAt = new Date();
      const lastInitialSyncAt = new Date();
      const isBlockChainSynced = false;

      const syncInfo = new SyncInfo(
        lastSyncedBlock.height,
        lastSyncedBlock.hash,
        lastSyncAt,
        lastInitialSyncAt,
        lastChainBlock.height,
        lastChainBlock.hash,
        isBlockChainSynced,
      );
      expect(syncInfo.getStatus()).to.be.equal(SyncInfo.STATUSES.SYNCING);
    });

    it('should be SYNCING if last synced block hash is different with last chain block hash', async () => {
      const lastSyncedBlock = blocks[0];
      const lastChainBlock = blocks[3];
      const lastSyncAt = new Date();
      const lastInitialSyncAt = new Date();
      const isBlockChainSynced = true;

      const syncInfo = new SyncInfo(
        lastSyncedBlock.height,
        lastSyncedBlock.hash,
        lastSyncAt,
        lastInitialSyncAt,
        lastChainBlock.height,
        lastChainBlock.hash,
        isBlockChainSynced,
      );
      expect(syncInfo.getStatus()).to.be.equal(SyncInfo.STATUSES.SYNCING);
    });

    it('should be synced if ChainInfo isBlockchainSynced is true and SyncState last block hash is equal to ChainInfo last block hash', async () => {
      const lastSyncedBlock = blocks[3];
      const lastChainBlock = blocks[3];
      const lastSyncAt = new Date();
      const lastInitialSyncAt = new Date();
      const isBlockChainSynced = true;

      const syncInfo = new SyncInfo(
        lastSyncedBlock.height,
        lastSyncedBlock.hash,
        lastSyncAt,
        lastInitialSyncAt,
        lastChainBlock.height,
        lastChainBlock.hash,
        isBlockChainSynced,
      );
      expect(syncInfo.getStatus()).to.be.equal(SyncInfo.STATUSES.SYNCED);
    });
  });
});
