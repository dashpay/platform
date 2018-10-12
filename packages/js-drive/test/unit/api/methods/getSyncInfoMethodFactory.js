const getBlockFixtures = require('../../../../lib/test/fixtures/getBlockFixtures');
const SyncInfo = require('../../../../lib/sync/info/SyncInfo');
const getSyncInfoMethodFactory = require('../../../../lib/api/methods/getSyncInfoMethodFactory');

describe('getSyncStatusMethodFactory', () => {
  let getSyncInfo;
  let getSyncInfoMethod;
  const blocks = getBlockFixtures();

  beforeEach(function beforeEach() {
    getSyncInfo = this.sinon.stub();
    getSyncInfoMethod = getSyncInfoMethodFactory(getSyncInfo);
  });

  it('should throw an error if getSyncStatus fails', async () => {
    getSyncInfo.throws(new Error());
    expect(getSyncInfoMethod()).to.be.rejectedWith(Error);
  });

  it('should return Sync Status', async () => {
    const lastSyncedBlock = blocks[0];
    const lastChainBlock = blocks[3];
    const lastSyncAt = new Date();
    const lastInitialSyncAt = new Date();
    const isBlockchainSynced = true;
    const syncInfo = new SyncInfo(
      lastSyncedBlock.height,
      lastSyncedBlock.hash,
      lastSyncAt,
      lastInitialSyncAt,
      lastChainBlock.height,
      lastChainBlock.hash,
      isBlockchainSynced,
    );
    getSyncInfo.returns(syncInfo);
    const syncStatusData = await getSyncInfoMethod();
    expect(syncStatusData).to.be.deep.equal({
      lastSyncedBlockHeight: lastSyncedBlock.height,
      lastSyncedBlockHash: lastSyncedBlock.hash,
      lastSyncAt,
      lastInitialSyncAt,
      lastChainBlockHeight: lastChainBlock.height,
      lastChainBlockHash: lastChainBlock.hash,
      status: SyncInfo.STATUSES.SYNCING,
    });
  });
});
