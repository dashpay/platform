const createClientWithoutWallet = require('../../../lib/test/createClientWithoutWallet');

describe('Platform', () => {
  describe('getStatus', () => {
    let client;

    before(async () => {
      client = await createClientWithoutWallet();
    });

    after(async () => {
      if (client) {
        await client.disconnect();
      }
    });

    it('should return status', async () => {
      const status = await client.dapiClient.platform.getStatus();
      const versionStatus = status.getVersionStatus();

      expect(versionStatus.getDapiVersion()).to.be.a('string').to.exist();
      expect(versionStatus.getDriveVersion()).to.be.a('string').and.not.be.empty();
      expect(versionStatus.getTenderdashVersion()).to.be.a('string').and.not.be.empty();
      expect(versionStatus.getDriveCurrentProtocol()).to.be.a('number').and.be.greaterThan(0);
      expect(versionStatus.getDriveLatestProtocol()).to.be.a('number').and.be.greaterThan(0);
      expect(versionStatus.getDriveNextEpochProtocol()).to.be.a('number').and.be.greaterThan(0);
    });
  });
});
