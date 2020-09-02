const {
  PrivateKey,
} = require('@dashevo/dashcore-lib');

const createFaucetClient = require('../../../lib/test/createFaucetClient');

describe('Core', () => {
  describe('broadcastTransaction', () => {
    let faucetClient;

    before(() => {
      faucetClient = createFaucetClient();
    });

    it('should sent transaction and return transaction ID', async () => {
      const faucetWalletAccount = await faucetClient.getWalletAccount();

      const transaction = faucetWalletAccount.createTransaction({
        recipient: new PrivateKey().toAddress(process.env.NETWORK),
        satoshis: 10000,
      });

      const dapiClient = faucetClient.getDAPIClient();

      const transactionId = await dapiClient.core.broadcastTransaction(transaction.serialize());

      expect(transactionId).to.be.a('string');
    });
  });
});
