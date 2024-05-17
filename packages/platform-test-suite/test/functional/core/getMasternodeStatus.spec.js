const { Essentials: { Buffer } } = require('dash');

const createClientWithoutWallet = require('../../../lib/test/createClientWithoutWallet');

describe('Core', () => {
  describe('getMasternodeStatus', function main() {
    let client;

    this.timeout(160000);

    before(() => {
      client = createClientWithoutWallet();
    });

    after(async () => {
      if (client) {
        await client.disconnect();
      }
    });

    it('should return status', async () => {
      const result = await client.getDAPIClient().core.getMasternodeStatus();

      const {
        status, proTxHash, posePenalty, isSynced, syncProgress,
      } = result;

      expect(status).to.be.a('string');
      expect(proTxHash).to.be.an.instanceOf(Buffer);
      expect(posePenalty).to.be.a('number');
      expect(isSynced).to.be.a('boolean');
      expect(syncProgress).to.be.a('number');
    });
  });
});
