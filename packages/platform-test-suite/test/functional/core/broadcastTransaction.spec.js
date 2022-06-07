const Dash = require('dash');

const createClientWithFundedWallet = require('../../../lib/test/createClientWithFundedWallet');

const { Core: { PrivateKey } } = Dash;

describe('Core', () => {
  describe('broadcastTransaction', () => {
    let client;

    before(async () => {
      client = await createClientWithFundedWallet();
    });

    after(async () => {
      await client.disconnect();
    });

    it('should sent transaction and return transaction ID', async () => {
      const account = await client.getWalletAccount();

      const transaction = account.createTransaction({
        recipient: new PrivateKey().toAddress(process.env.NETWORK),
        satoshis: 10000,
      });

      const dapiClient = client.getDAPIClient();

      const transactionId = await dapiClient.core.broadcastTransaction(transaction.toBuffer());

      expect(transactionId).to.be.a('string');
    });
  });
});
