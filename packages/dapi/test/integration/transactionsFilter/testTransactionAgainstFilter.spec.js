const { mocha: { startDashCore } } = require('@dashevo/dp-services-ctl');

const {
  Transaction,
  PrivateKey,
  BloomFilter,
  Address,
  Networks,
  MerkleBlock,
} = require('@dashevo/dashcore-lib');

const testTransactionAgainstFilter = require('../../../lib/transactionsFilter/testTransactionAgainstFilter');

describe('testTransactionAgainstFilter', () => {
  let coreApi;

  startDashCore().then((core) => {
    coreApi = core.getApi();
  });

  it('should match the same transaction as Core', async () => {
    // Create a transactions
    const { result: addressBase58 } = await coreApi.getnewaddress();
    const { result: privateKeyString } = await coreApi.dumpprivkey(addressBase58);

    const address = Address.fromString(addressBase58, Networks.testnet);
    const privateKey = new PrivateKey(privateKeyString);

    await coreApi.generate(101);
    await coreApi.sendtoaddress(addressBase58, 10);
    await coreApi.generate(7);

    const { result: unspent } = await coreApi.listunspent();
    const inputs = unspent.filter(input => input.address === addressBase58);

    const transaction = new Transaction()
      .from(inputs)
      .to(address, 10000)
      .change(address)
      .sign(privateKey);

    // Create a bloom filter
    const filter = BloomFilter.create(1, 0.0001);
    filter.insert(address.hashBuffer);

    // Test transaction with `testTransactionAgainstFilter` function
    const result = testTransactionAgainstFilter(filter, transaction);
    expect(result).to.be.true();

    // Test transaction with Core
    await coreApi.sendrawtransaction(transaction.serialize());

    await coreApi.generate(1);

    const { result: firstBlockHash } = await coreApi.getBlockHash(1);

    const { result: merkleBlockStrings } = await coreApi.getMerkleBlocks(
      filter.toBuffer().toString('hex'),
      firstBlockHash,
    );

    expect(merkleBlockStrings).to.be.an('array');

    const merkleBlockWithTransaction = merkleBlockStrings
      .map(merkleBlockString => new MerkleBlock(Buffer.from(merkleBlockString, 'hex')))
      .find(merkleBlock => merkleBlock.hasTransaction(transaction));

    expect(merkleBlockWithTransaction).to.be.instanceOf(MerkleBlock);
    expect(merkleBlockWithTransaction.hasTransaction(transaction)).to.be.true();
  });
});
