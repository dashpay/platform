const {
  PrivateKey,
} = require('@dashevo/dashcore-lib');

const createClientWithoutWallet = require('../../../lib/test/createClientWithoutWallet');

describe('Core', () => {
  describe('getAddressSummary', () => {
    let address;
    let client;

    before(() => {
      client = createClientWithoutWallet();
    });

    after(async () => {
      if (client) {
        await client.disconnect();
      }
    });

    it('should return address summary', async () => {
      address = new PrivateKey()
        .toAddress(process.env.NETWORK)
        .toString();

      const result = await client.getDAPIClient().core.getAddressSummary(address);

      expect(result).to.be.an('object');
      expect(result.addrStr).to.equal(address);
    });

    it('should throw an error on invalid params', async () => {
      address = 'Xh7nD4vTUYAxy8GV7t1k8Er9ZKmxRBDcL';

      try {
        await client.getDAPIClient().core.getAddressSummary(address);

        expect.fail('should throw an error');
      } catch (e) {
        expect(e.name).to.equal('JsonRpcError');
        expect(e.message).contains('Invalid address');
      }
    });
  });
});
