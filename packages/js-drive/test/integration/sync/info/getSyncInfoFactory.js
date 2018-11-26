const { mocha: { startMongoDb, startDashCore } } = require('@dashevo/js-evo-services-ctl');
const getBlockFixtures = require('../../../../lib/test/fixtures/getBlockFixtures');

const SyncState = require('../../../../lib/sync/state/SyncState');
const getSyncInfoFactory = require('../../../../lib/sync/info/getSyncInfoFactory');
const SyncStateRepository = require('../../../../lib/sync/state/repository/SyncStateRepository');
const getChainInfoFactory = require('../../../../lib/sync/info/chain/getChainInfoFactory');

describe('getSyncInfoFactory', () => {
  let mongoDb;
  startMongoDb().then(async (mongoInstance) => {
    mongoDb = await mongoInstance.getDb();
  });

  let rpcClient;
  startDashCore().then((dashCoreInstance) => {
    rpcClient = dashCoreInstance.getApi();
  });

  let blocks;
  let syncStateRepository;
  let getSyncInfo;
  beforeEach(() => {
    blocks = getBlockFixtures();
    syncStateRepository = new SyncStateRepository(mongoDb);
    const getChainInfo = getChainInfoFactory(rpcClient);
    getSyncInfo = getSyncInfoFactory(syncStateRepository, getChainInfo);
  });

  it('should be initialSync if there is no SyncState yet', async () => {
    const syncInfo = await getSyncInfo();
    expect(syncInfo.toJSON()).to.be.deep.equal({
      lastSyncedBlockHeight: undefined,
      lastSyncedBlockHash: undefined,
      lastSyncAt: null,
      lastInitialSyncAt: null,
      lastChainBlockHeight: 0,
      lastChainBlockHash: '000008ca1832a4baf228eb1553c03d3a2c8e02399550dd6ea8d65cec3ef23d2e',
      status: 'initialSync',
    });
  });

  it('should be initialSync if SyncState lastInitialSyncAt does not exist', async () => {
    const lastSyncedBlock = blocks[1];
    const lastSyncAt = new Date();
    const lastInitialSyncAt = null;
    const syncState = new SyncState([lastSyncedBlock], lastSyncAt, lastInitialSyncAt);
    await syncStateRepository.store(syncState);

    const syncInfo = await getSyncInfo();
    expect(syncInfo.toJSON()).to.be.deep.equal({
      lastSyncedBlockHeight: lastSyncedBlock.height,
      lastSyncedBlockHash: lastSyncedBlock.hash,
      lastSyncAt,
      lastInitialSyncAt,
      lastChainBlockHeight: 0,
      lastChainBlockHash: '000008ca1832a4baf228eb1553c03d3a2c8e02399550dd6ea8d65cec3ef23d2e',
      status: 'initialSync',
    });
  });

  it('should be syncing if SyncState lastInitialSyncAt already exists', async () => {
    const lastSyncedBlock = blocks[1];
    const lastSyncAt = new Date();
    const lastInitialSyncAt = new Date();
    const syncState = new SyncState([lastSyncedBlock], lastSyncAt, lastInitialSyncAt);
    await syncStateRepository.store(syncState);

    const syncInfo = await getSyncInfo();
    expect(syncInfo.toJSON()).to.be.deep.equal({
      lastSyncedBlockHeight: lastSyncedBlock.height,
      lastSyncedBlockHash: lastSyncedBlock.hash,
      lastSyncAt,
      lastInitialSyncAt,
      lastChainBlockHeight: 0,
      lastChainBlockHash: '000008ca1832a4baf228eb1553c03d3a2c8e02399550dd6ea8d65cec3ef23d2e',
      status: 'syncing',
    });
  });

  it('should have lastChainBlock info if new blocks are generated', async () => {
    const lastSyncedBlock = blocks[1];
    const lastSyncAt = new Date();
    const lastInitialSyncAt = new Date();
    const syncState = new SyncState([lastSyncedBlock], lastSyncAt, lastInitialSyncAt);
    await syncStateRepository.store(syncState);

    const { result: chainBlocksHashes } = await rpcClient.generate(20);
    const lastChainBlockHash = chainBlocksHashes[chainBlocksHashes.length - 1];

    const syncInfo = await getSyncInfo();
    expect(syncInfo.toJSON()).to.be.deep.equal({
      lastSyncedBlockHeight: lastSyncedBlock.height,
      lastSyncedBlockHash: lastSyncedBlock.hash,
      lastSyncAt,
      lastInitialSyncAt,
      lastChainBlockHeight: chainBlocksHashes.length,
      lastChainBlockHash,
      status: 'syncing',
    });
  });
});
