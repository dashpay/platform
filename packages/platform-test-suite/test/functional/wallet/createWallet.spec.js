const createClientWithFundedWallet = require('../../../lib/test/createClientWithFundedWallet');

describe('Wallet', () => {
  describe('Create', function describeIdentity() {
    this.bail(true); // bail on first failure

    let client;
    let identity;
    let walletAccount;

    before(async () => {

      walletAccount = await client.getWalletAccount();
    });

    after(async () => {
      if (client) {
        await client.disconnect();
      }
    });

    it('should create an identity', async () => {
      await Promise.all([
        createClientWithFundedWallet(10000),
        createClientWithFundedWallet(10000),
        createClientWithFundedWallet(10000),
        createClientWithFundedWallet(10000),
        createClientWithFundedWallet(10000),
        createClientWithFundedWallet(10000),
        createClientWithFundedWallet(10000),
        createClientWithFundedWallet(10000),
        createClientWithFundedWallet(10000),
        createClientWithFundedWallet(10000),
        createClientWithFundedWallet(10000),
        createClientWithFundedWallet(10000),
        createClientWithFundedWallet(10000),
        createClientWithFundedWallet(10000)
      ])

      expect(identity).to.exist();
    });
  });
});
