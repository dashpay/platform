const {
  Transaction,
  PrivateKey,
} = require('@dashevo/dashcore-lib');

const NotFoundError = require('@dashevo/dapi-client/lib/methods/errors/NotFoundError');

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
      const receivedTx = new Transaction(result.getTransaction());

      expect(receivedTx.hash).to.deep.equal(transaction.id);
    });

    it('should throw NotFound error if transaction was not found', async () => {
      const nonExistentId = Buffer.alloc(32).toString('hex');

      try {
        await faucetClient.getDAPIClient().core.getTransaction(nonExistentId);

        expect.fail('should throw NotFound');
      } catch (e) {
        expect(e).to.be.an.instanceOf(NotFoundError);
      }
    });
  });
});
