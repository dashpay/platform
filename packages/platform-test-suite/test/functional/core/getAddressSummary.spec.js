const {
  PrivateKey,
} = require('@dashevo/dashcore-lib');

const fundAddress = require('../../../lib/test/fundAddress');

describe.skip('Core', () => {
  describe('getAddressSummary', () => {
    let address;

    before(async () => {
      const faucetPrivateKey = PrivateKey.fromString(process.env.FAUCET_PRIVATE_KEY);
      const faucetAddress = faucetPrivateKey
        .toAddress(process.env.NETWORK)
        .toString();

      address = new PrivateKey()
        .toAddress(process.env.NETWORK)
        .toString();

      await fundAddress(
        dashClient.getDAPIClient(),
        faucetAddress,
        faucetPrivateKey,
        address,
        20000,
      );
    });

    it('should return address summary', async () => {
      const result = await dashClient.clients.dapi.getAddressSummary(address);

      expect(result).to.be.an('object');
      expect(result.addrStr).to.equal(address);
    });

    it('should throw an error on invalid params', async () => {
      address = 'Xh7nD4vTUYAxy8GV7t1k8Er9ZKmxRBDcL';

      try {
        await dashClient.clients.dapi.getAddressSummary(address);

        expect.fail('should throw an error');
      } catch (e) {
        expect(e.name).to.equal('RPCError');
        expect(e.message).contains('Invalid address');
      }
    });
  });
});
