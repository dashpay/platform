const {
  Transaction,
  PrivateKey,
} = require('@dashevo/dashcore-lib');

const createClientWithoutWallet = require('../../../lib/test/createClientWithoutWallet');

const fundAddress = require('../../../lib/test/fundAddress');

describe('Core', () => {
  describe('getTransaction', () => {
    let client;

    before(() => {
      client = createClientWithoutWallet();
    });

    after(async () => {
      if (client) {
        await client.disconnect();
      }
    });

    it('should respond with a transaction by it\'s ID', async () => {
      const faucetPrivateKey = PrivateKey.fromString(process.env.FAUCET_PRIVATE_KEY);
      const faucetAddress = faucetPrivateKey
        .toAddress(process.env.NETWORK)
        .toString();

      const address = new PrivateKey()
        .toAddress(process.env.NETWORK)
        .toString();

      const transactionId = await fundAddress(
        client.getDAPIClient(),
        faucetAddress,
        faucetPrivateKey,
        address,
        20000,
      );

      const result = await client.getDAPIClient().core.getTransaction(transactionId);
      const receivedTx = new Transaction(Buffer.from(result));

      expect(receivedTx.hash).to.deep.equal(transactionId);
    });

    it('should respond with null if transaction was not found', async () => {
      const nonExistentId = Buffer.alloc(32).toString('hex');

      const result = await client.getDAPIClient().core.getTransaction(nonExistentId);

      expect(result).to.equal(null);
    });
  });
});
