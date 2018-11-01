const getBlockFixtures = require('../../../../lib/test/fixtures/getBlockFixtures');
const SyncState = require('../../../../lib/sync/state/SyncState');
const SyncInfo = require('../../../../lib/sync/info/SyncInfo');
const ChainInfo = require('../../../../lib/sync/info/chain/ChainInfo');
const getSyncInfoFactory = require('../../../../lib/sync/info/getSyncInfoFactory');

describe('getSyncInfoFactory', () => {
  let blocks;
  let syncStateRepository;
  let getChainInfo;
  let lastChainBlock;
  let getSyncInfo;

  beforeEach(function beforeEach() {
    blocks = getBlockFixtures();
    // eslint-disable-next-line prefer-destructuring
    lastChainBlock = blocks[3];
    syncStateRepository = {
      fetch: this.sinon.stub(),
    };
    getChainInfo = this.sinon.stub();
    const isBlockchainSynced = true;
    const chainInfo = new ChainInfo(lastChainBlock.height, lastChainBlock.hash, isBlockchainSynced);
    getChainInfo.returns(chainInfo);
    getSyncInfo = getSyncInfoFactory(syncStateRepository, getChainInfo);
  });

  describe('lastSyncAt', () => {
    it('should be null if SyncState does not have lastSyncAt', async () => {
      const syncStateLastSyncAt = null;
      const syncState = new SyncState(blocks, syncStateLastSyncAt);
      syncStateRepository.fetch.returns(syncState);
      const syncInfo = await getSyncInfo();
      expect(syncInfo.getLastSyncAt()).to.be.deep.equal(syncStateLastSyncAt);
    });

    it('should be equal to SyncState lastSyncAt', async () => {
      const syncStateLastSyncAt = new Date();
      const syncState = new SyncState(blocks, syncStateLastSyncAt);
      syncStateRepository.fetch.returns(syncState);
      const syncInfo = await getSyncInfo();
      expect(syncInfo.getLastSyncAt()).to.be.deep.equal(syncStateLastSyncAt);
    });
  });

  describe('lastInitialSyncAt', () => {
    it('should be null if SyncState does not have lastInitialSyncAt', async () => {
      const syncStateLastSyncAt = null;
      const syncStateLastInitialSyncAt = null;
      const syncState = new SyncState(blocks, syncStateLastSyncAt, syncStateLastInitialSyncAt);
      syncStateRepository.fetch.returns(syncState);
      const syncInfo = await getSyncInfo();
      expect(syncInfo.getLastInitialSyncAt()).to.be.deep.equal(syncStateLastSyncAt);
    });

    it('should be equal to SyncState lastInitialSyncAt', async () => {
      const syncStateLastSyncAt = new Date();
      const syncStateLastInitialSyncAt = new Date();
      const syncState = new SyncState(blocks, syncStateLastSyncAt, syncStateLastInitialSyncAt);
      syncStateRepository.fetch.returns(syncState);
      const syncInfo = await getSyncInfo();
      expect(syncInfo.getLastInitialSyncAt()).to.be.deep.equal(syncStateLastInitialSyncAt);
    });
  });

  describe('status', () => {
    it('should be initialSync if SyncState hash not lastSyncAt', async () => {
      const syncStateLastSyncAt = null;
      const syncState = new SyncState(blocks, syncStateLastSyncAt);
      syncStateRepository.fetch.returns(syncState);
      const isBlockchainSynced = false;
      const chainInfo = new ChainInfo(
        lastChainBlock.height,
        lastChainBlock.hash,
        isBlockchainSynced,
      );
      getChainInfo.returns(chainInfo);
      const syncInfo = await getSyncInfo();
      expect(syncInfo.getStatus()).to.be.deep.equal(SyncInfo.STATUSES.INITIAL_SYNC);
    });

    it('should be syncing if SyncState has lastSyncAt, lastInitialSyncAt and isBlockchainSynced is false', async () => {
      const syncStateLastSyncAt = new Date();
      const syncLastInitialSyncAt = new Date();
      const syncState = new SyncState(blocks, syncStateLastSyncAt, syncLastInitialSyncAt);
      syncStateRepository.fetch.returns(syncState);
      const isBlockchainSynced = false;
      const chainInfo = new ChainInfo(
        lastChainBlock.height,
        lastChainBlock.hash,
        isBlockchainSynced,
      );
      getChainInfo.returns(chainInfo);
      const syncInfo = await getSyncInfo();
      expect(syncInfo.getStatus()).to.be.deep.equal(SyncInfo.STATUSES.SYNCING);
    });

    it('should be synced if SyncState last block hash is equal to ChainInfo last block hash', async () => {
      const syncStateLastSyncAt = new Date();
      const syncStateLastInitialSyncAt = new Date();
      const syncState = new SyncState(blocks, syncStateLastSyncAt, syncStateLastInitialSyncAt);
      syncStateRepository.fetch.returns(syncState);
      const isBlockchainSynced = true;
      const chainInfo = new ChainInfo(
        lastChainBlock.height,
        lastChainBlock.hash,
        isBlockchainSynced,
      );
      getChainInfo.returns(chainInfo);
      const syncInfo = await getSyncInfo();
      expect(syncInfo.getStatus()).to.be.deep.equal(SyncInfo.STATUSES.SYNCED);
    });
  });

  describe('lastSyncedBlockHeight', () => {
    it('should be the same block height as in SyncState', async () => {
      const syncStateLastSyncAt = new Date();
      const syncState = new SyncState(blocks, syncStateLastSyncAt);
      syncStateRepository.fetch.returns(syncState);
      const syncInfo = await getSyncInfo();
      expect(syncInfo.getLastSyncedBlockHeight()).to.be.deep.equal(lastChainBlock.height);
    });
  });

  describe('lastSyncedBlockHash', () => {
    it('should be the same block hash as in SyncState', async () => {
      const syncStateLastSyncAt = new Date();
      const syncState = new SyncState(blocks, syncStateLastSyncAt);
      syncStateRepository.fetch.returns(syncState);
      const syncInfo = await getSyncInfo();
      expect(syncInfo.getLastSyncedBlockHash()).to.be.deep.equal(lastChainBlock.hash);
    });
  });

  describe('lastChainBlockHeight', () => {
    it('should be the same block hash as the one returned by getChainInfo', async () => {
      const syncStateLastSyncAt = new Date();
      const syncState = new SyncState(blocks, syncStateLastSyncAt);
      syncStateRepository.fetch.returns(syncState);
      const isBlockchainSynced = true;
      getChainInfo.returns(new ChainInfo(
        lastChainBlock.height,
        lastChainBlock.hash,
        isBlockchainSynced,
      ));
      const syncInfo = await getSyncInfo();
      expect(syncInfo.getLastChainBlockHeight()).to.be.deep.equal(lastChainBlock.height);
    });
  });

  describe('lastChainBlockHash', () => {
    it('should be the same block height as the one returned by getChainInfo', async () => {
      const syncStateLastSyncAt = new Date();
      const syncState = new SyncState(blocks, syncStateLastSyncAt);
      syncStateRepository.fetch.returns(syncState);
      const isBlockchainSynced = true;
      getChainInfo.returns(new ChainInfo(
        lastChainBlock.height,
        lastChainBlock.hash,
        isBlockchainSynced,
      ));
      const syncInfo = await getSyncInfo();
      expect(syncInfo.getLastChainBlockHash()).to.be.deep.equal(lastChainBlock.hash);
    });
  });
});
