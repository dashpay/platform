const createClientWithoutWallet = require('../../../lib/test/createClientWithoutWallet');

describe('Core', () => {
  describe('getStatus', function main() {
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
      const result = await client.getDAPIClient().getStatus();

      expect(result).to.have.a.property('coreVersion');
      expect(result).to.have.a.property('protocolVersion');
      expect(result).to.have.a.property('blocks');
      expect(result).to.have.a.property('timeOffset');
      expect(result).to.have.a.property('connections');
      expect(result).to.have.a.property('proxy');
      expect(result).to.have.a.property('difficulty');
      expect(result).to.have.a.property('testnet');
      expect(result).to.have.a.property('relayFee');
      expect(result).to.have.a.property('errors');
      expect(result).to.have.a.property('network');

      expect(result.blocks).to.be.a('number');
    });
  });
});
