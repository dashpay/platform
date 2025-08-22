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

      expect(status.getVersionStatus().getDapiVersion()).to.be.a('string').to.exist();
      expect(status.getVersionStatus().getDriveVersion()).to.be.a('string').and.not.be.empty();
      expect(status.getVersionStatus().getTenderdashVersion()).to.be.a('string').and.not.be.empty();
    });
  });
});
