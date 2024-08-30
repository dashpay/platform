const createClientWithoutWallet = require('../../../lib/test/createClientWithoutWallet');

describe.only('Platform', () => {
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

      console.dir(status, { depth: null });
    });
  });
});
