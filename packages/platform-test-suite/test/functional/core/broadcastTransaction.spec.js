const {
  Transaction,
  PrivateKey,
} = require('@dashevo/dashcore-lib');

const createClientWithoutWallet = require('../../../lib/test/createClientWithoutWallet');
const getInputsByAddress = require('../../../lib/test/getInputsByAddress');

describe('Core', () => {
  describe('broadcastTransaction', () => {
    let client;

    before(() => {
      client = createClientWithoutWallet();
    });

    after(async () => {
      if (client) {
        await client.disconnect();
      }
    });

    it('should sent transaction and return transaction ID', async () => {
      const faucetPrivateKey = PrivateKey.fromString(process.env.FAUCET_PRIVATE_KEY);
      const faucetAddress = faucetPrivateKey
        .toAddress(process.env.NETWORK)
        .toString();

      const address = new PrivateKey()
        .toAddress(process.env.NETWORK)
        .toString();

      const amount = 10000;

      const inputs = await getInputsByAddress(client.getDAPIClient(), faucetAddress);

      const transaction = new Transaction();

      transaction.from(inputs.slice(-1)[0])
        .to(address, amount)
        .change(faucetAddress)
        .sign(faucetPrivateKey);

      const serializedTransaction = Buffer.from(transaction.serialize(), 'hex');

      const result = await client.getDAPIClient().sendTransaction(serializedTransaction);

      expect(result).to.be.a('string');
    });
  });
});
