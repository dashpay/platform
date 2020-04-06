const startDapi = require('@dashevo/dp-services-ctl/lib/services/startDapi');

const {
  Transaction,
  PrivateKey,
} = require('@dashevo/dashcore-lib');

describe('getTransactionHandlerFactor', function main() {
  this.timeout(260000);

  let removeDapi;
  let dapiClient;
  let transaction;
  let transactionId;

  beforeEach(async () => {
    const {
      dapiCore,
      dashCore,
      remove,
    } = await startDapi();

    removeDapi = remove;

    dapiClient = dapiCore.getApi();

    const coreAPI = dashCore.getApi();

    const { result: fromAddress } = await coreAPI.getNewAddress();
    const { result: privateKeyString } = await coreAPI.dumpPrivKey(fromAddress);
    const privateKey = new PrivateKey(privateKeyString);

    const { result: toAddress } = await coreAPI.getNewAddress();

    await coreAPI.generateToAddress(1000, fromAddress);

    const { items: unspent } = await dapiClient.getUTXO(fromAddress);

    const amount = 10000;

    transaction = new Transaction();

    transaction.from(unspent.slice(-1)[0])
      .to(toAddress, amount)
      .change(fromAddress)
      .fee(1000)
      .sign(privateKey);

    ({ result: transactionId } = await coreAPI.sendRawTransaction(transaction.serialize()));
  });

  afterEach(async () => {
    await removeDapi();
  });

  it('should respond with a transaction by it\'s ID', async () => {
    const result = await dapiClient.getTransaction(transactionId);
    const receivedTx = new Transaction(Buffer.from(result));
    expect(receivedTx.toString('hex')).to.deep.equal(transaction.serialize());
  });

  it('should respond with null if transaction was not found', async () => {
    const nonExistentId = Buffer.alloc(32).toString('hex');

    const result = await dapiClient.getTransaction(nonExistentId);

    expect(result).to.be.null();
  });
});
