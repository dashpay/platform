const {
  Transaction,
  PrivateKey,
} = require('@dashevo/dashcore-lib');

describe.skip('Core', () => {
  let transaction;

  before(async () => {
    const faucetPrivateKey = PrivateKey.fromString(process.env.FAUCET_PRIVATE_KEY);
    const faucetAddress = faucetPrivateKey
      .toAddress(process.env.NETWORK)
      .toString();

    const address = new PrivateKey()
      .toAddress(process.env.NETWORK)
      .toString();

    const { blocks } = await dashClient.clients.dapi.getStatus();

    const { items: utxos } = await dashClient.clients.dapi.getUTXO(faucetAddress);

    const amount = 10000;

    const sortedUtxos = utxos
      .filter((utxo) => utxo.height < blocks - 100)
      .sort((a, b) => a.satoshis > b.satoshis);

    const inputs = [];
    let sum = 0;
    let i = 0;

    do {
      const input = sortedUtxos[i];
      inputs.push(input);
      sum += input.satoshis;

      ++i;
    } while (sum < amount && i < sortedUtxos.length);

    transaction = new Transaction();

    transaction.from(inputs.slice(-1)[0])
      .to(address, amount)
      .change(faucetAddress)
      .fee(668)
      .sign(faucetPrivateKey);
  });

  describe('sendTransaction', () => {
    it('should sent transaction and return transaction ID', async () => {
      const options = {};

      const result = await dashClient.clients.dapi.sendTransaction(Buffer.from(transaction.serialize(), 'hex'), options);

      expect(result).to.be.a('string');
    });
  });
});
