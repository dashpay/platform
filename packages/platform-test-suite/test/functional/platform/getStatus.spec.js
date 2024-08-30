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

      expect(status).to.be.a.property('version');
      expect(status.version).to.have.property('software');
      expect(status.version.software).to.have.an('object');
      expect(status.version.software.dapi).to.be.a('string').and.not.be.empty();
      expect(status.version.software.drive).to.be.a('string').and.not.be.empty();
      expect(status.version.software.tenderdash).to.be.a('string').and.not.be.empty();
    });
  });
});
