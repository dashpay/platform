const Dash = require('dash');

const wait = require('../../../lib/wait');
const createClientWithFundedWallet = require('../../../lib/test/createClientWithFundedWallet');

const { Core: { Transaction, PrivateKey } } = Dash;

describe('Core', () => {
  describe('getTransaction', () => {
    let client;

    before(async () => {
      client = await createClientWithFundedWallet();
    });

    after(async () => {
      await client.disconnect();
    });

    it('should respond with a transaction by it\'s ID', async () => {
      const account = await client.getWalletAccount();

      await wait(5000);

      const transaction = account.createTransaction({
        recipient: new PrivateKey().toAddress(process.env.NETWORK),
        satoshis: 10000,
      });

      await account.broadcastTransaction(transaction);

      await wait(5000);

      const result = await client.getDAPIClient().core.getTransaction(transaction.id);
      const receivedTx = new Transaction(result.getTransaction());

      expect(receivedTx.hash).to.deep.equal(transaction.id);
    });

    it('should throw NotFound error if transaction was not found', async () => {
      const nonExistentId = Buffer.alloc(32).toString('hex');

      try {
        await client.getDAPIClient().core.getTransaction(nonExistentId);

        expect.fail('should throw NotFound');
      } catch (e) {
        expect(e.constructor.name === 'NotFoundError');
      }
    });
  });
});
