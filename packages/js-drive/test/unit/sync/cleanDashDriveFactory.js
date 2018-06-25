const cleanDashDriveFactory = require('../../../lib/sync/cleanDashDriveFactory');

describe('cleanDashDriveFactory', () => {
  let unpinAllPacketsSpy;
  let dropMongoDatabasesWithPrefixSpy;
  beforeEach(function beforeEach() {
    unpinAllPacketsSpy = this.sinon.spy();
    dropMongoDatabasesWithPrefixSpy = this.sinon.spy();
  });

  it('should clean DashDrive', async () => {
    const cleanDashDrive = cleanDashDriveFactory(
      unpinAllPacketsSpy,
      dropMongoDatabasesWithPrefixSpy,
    );
    await cleanDashDrive();

    expect(unpinAllPacketsSpy).to.calledOnce();
    expect(dropMongoDatabasesWithPrefixSpy).to.calledOnce();
  });
});
