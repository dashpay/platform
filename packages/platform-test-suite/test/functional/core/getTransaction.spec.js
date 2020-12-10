const {
  Transaction,
  PrivateKey,
} = require('@dashevo/dashcore-lib');

const createFaucetClient = require('../../../lib/test/createFaucetClient');
const wait = require('../../../lib/wait');

describe('Core', () => {
  describe('getTransaction', () => {
    let faucetClient;

    before(() => {
      faucetClient = createFaucetClient();
    });

    it('should respond with a transaction by it\'s ID', async () => {
      const faucetWalletAccount = await faucetClient.getWalletAccount();

      await wait(5000);

      const transaction = faucetWalletAccount.createTransaction({
        recipient: new PrivateKey().toAddress(process.env.NETWORK),
        satoshis: 10000,
      });

      await faucetWalletAccount.broadcastTransaction(transaction);

      const result = await faucetClient.getDAPIClient().core.getTransaction(transaction.id);
      const receivedTx = new Transaction(Buffer.from(result));

      expect(receivedTx.hash).to.deep.equal(transaction.id);
    });

    it('should respond with null if transaction was not found', async () => {
      const nonExistentId = Buffer.alloc(32).toString('hex');

      const result = await faucetClient.getDAPIClient().core.getTransaction(nonExistentId);

      expect(result).to.equal(null);
    });
  });
});
