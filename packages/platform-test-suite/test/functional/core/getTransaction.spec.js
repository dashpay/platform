const {
  Transaction,
  PrivateKey,
} = require('@dashevo/dashcore-lib');

const fundAddress = require('../../../lib/test/fundAddress');

describe.skip('Core', () => {
  describe('getTransaction', () => {
    let transactionId;

    before(async () => {
      const faucetPrivateKey = PrivateKey.fromString(process.env.FAUCET_PRIVATE_KEY);
      const faucetAddress = faucetPrivateKey
        .toAddress(process.env.NETWORK)
        .toString();

      const address = new PrivateKey()
        .toAddress(process.env.NETWORK)
        .toString();

      transactionId = await fundAddress(
        dashClient.clients.dapi,
        faucetAddress,
        faucetPrivateKey,
        address,
        20000,
      );
    });

    it('should respond with a transaction by it\'s ID', async () => {
      const result = await dashClient.clients.dapi.getTransaction(transactionId);
      const receivedTx = new Transaction(Buffer.from(result));

      expect(receivedTx.hash).to.deep.equal(transactionId);
    });

    it('should respond with null if transaction was not found', async () => {
      const nonExistentId = Buffer.alloc(32).toString('hex');

      const result = await dashClient.clients.dapi.getTransaction(nonExistentId);

      expect(result).to.equal(null);
    });
  });
});
