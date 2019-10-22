const {
  Transaction,
  Block,
  BlockHeader,
  MerkleBlock,
  PrivateKey,
  BloomFilter,
  util: { buffer: BufferUtils },
} = require('@dashevo/dashcore-lib');

const BloomFilterEmitterCollection = require('../../../lib/bloomFilter/emitter/BloomFilterEmitterCollection');
const ProcessMediator = require('../../../lib/transactionsFilter/ProcessMediator');

const subscribeToNewTransactions = require('../../../lib/transactionsFilter/subscribeToNewTransactions');
const testTransactionsAgainstFilter = require('../../../lib/transactionsFilter/testTransactionAgainstFilter');

/**
 * Reverse the hash
 *
 * @param {string} hash
 * @returns {string}
 */
function reverseHash(hash) {
  return BufferUtils.reverse(
    Buffer.from(hash, 'hex'),
  ).toString('hex');
}

describe('subscribeToNewTransactions', () => {
  let bloomFilter;
  let bloomFilterEmitterCollection;
  let mediator;
  let transactions;
  let blocks;

  beforeEach(() => {
    const address = new PrivateKey().toAddress();
    const anotherAddress = new PrivateKey().toAddress();

    transactions = [];
    transactions.push(new Transaction().to(address, 41));
    transactions.push(new Transaction().to(address, 42));
    transactions.push(new Transaction().to(anotherAddress, 43));

    transactions.push(new Transaction().to(address, 77));
    transactions.push(new Transaction().to(anotherAddress, 78));

    const blockHeaderOne = new BlockHeader({
      version: 536870913,
      prevHash: '0000000000000000000000000000000000000000000000000000000000000000',
      merkleRoot: 'c4970326400177ce67ec582425a698b85ae03cae2b0d168e87eed697f1388e4b',
      time: 1507208925,
      timestamp: 1507208645,
      bits: '1d00dda1',
      nonce: 1449878272,
    });

    const blockOne = new Block({
      header: blockHeaderOne.toObject(),
      transactions: [transactions[0], transactions[1], transactions[2]],
    });

    const blockHeaderTwo = new BlockHeader({
      version: 536870913,
      prevHash: blockOne.hash,
      merkleRoot: 'c4970326400177ce67ec582425a698b85ae03cae2b0d168e87eed697f1388e4c',
      time: 1507208926,
      timestamp: 1507208645,
      bits: '1d00dda1',
      nonce: 1449878272,
    });

    const blockTwo = new Block({
      header: blockHeaderTwo.toObject(),
      transactions: [transactions[3], transactions[4]],
    });

    blocks = [];
    blocks.push(blockOne);
    blocks.push(blockTwo);

    bloomFilter = BloomFilter.create(1, 0.0001);
    bloomFilter.insert(address.hashBuffer);

    bloomFilterEmitterCollection = new BloomFilterEmitterCollection();
    mediator = new ProcessMediator();
  });

  it('should add transactions and blocks in cache and send them back when historical data is sent', () => {
    const receivedTransactions = [];
    const receivedBlocks = [];

    mediator.on(ProcessMediator.EVENTS.TRANSACTION, (tx) => {
      receivedTransactions.push(tx);
    });

    mediator.on(ProcessMediator.EVENTS.MERKLE_BLOCK, (merkleBlock) => {
      receivedBlocks.push(merkleBlock);
    });

    subscribeToNewTransactions(
      mediator,
      bloomFilter,
      testTransactionsAgainstFilter,
      bloomFilterEmitterCollection,
    );

    bloomFilterEmitterCollection.test(transactions[0]);
    bloomFilterEmitterCollection.test(transactions[1]);
    bloomFilterEmitterCollection.test(transactions[2]);

    bloomFilterEmitterCollection.emit('block', blocks[0]);

    mediator.emit(ProcessMediator.EVENTS.HISTORICAL_DATA_SENT);
    mediator.emit(ProcessMediator.EVENTS.CLIENT_DISCONNECTED);

    expect(receivedTransactions).to.deep.equal([
      transactions[0],
      transactions[1],
    ]);

    const expectedMerkleBlock = MerkleBlock.build(
      blocks[0].header,
      [
        Buffer.from(transactions[0].hash, 'hex'),
        Buffer.from(transactions[1].hash, 'hex'),
        Buffer.from(transactions[2].hash, 'hex'),
      ],
      [true, true, false],
    );

    expectedMerkleBlock.hashes = expectedMerkleBlock.hashes
      .map(hash => reverseHash(hash));

    expect(receivedBlocks).to.have.a.lengthOf(1);
    expect(receivedBlocks[0]).to.deep.equal(expectedMerkleBlock);
  });

  it('should scan block for matching transactions if it is the first one arrived', () => {
    const receivedTransactions = [];
    const receivedBlocks = [];

    mediator.on(ProcessMediator.EVENTS.TRANSACTION, (tx) => {
      receivedTransactions.push(tx);
    });

    mediator.on(ProcessMediator.EVENTS.MERKLE_BLOCK, (merkleBlock) => {
      receivedBlocks.push(merkleBlock);
    });

    subscribeToNewTransactions(
      mediator,
      bloomFilter,
      testTransactionsAgainstFilter,
      bloomFilterEmitterCollection,
    );

    bloomFilterEmitterCollection.test(transactions[2]);

    bloomFilterEmitterCollection.emit('block', blocks[0]);

    mediator.emit(ProcessMediator.EVENTS.HISTORICAL_DATA_SENT);
    mediator.emit(ProcessMediator.EVENTS.CLIENT_DISCONNECTED);

    expect(receivedTransactions).to.deep.equal([
      transactions[0],
      transactions[1],
    ]);

    const expectedMerkleBlock = MerkleBlock.build(
      blocks[0].header,
      [
        Buffer.from(transactions[0].hash, 'hex'),
        Buffer.from(transactions[1].hash, 'hex'),
        Buffer.from(transactions[2].hash, 'hex'),
      ],
      [true, true, false],
    );

    expectedMerkleBlock.hashes = expectedMerkleBlock.hashes
      .map(hash => reverseHash(hash));

    expect(receivedBlocks).to.have.a.lengthOf(1);
    expect(receivedBlocks[0]).to.deep.equal(expectedMerkleBlock);
  });

  it('should remove historical data from cache and send only data that is left', () => {
    const receivedTransactions = [];
    const receivedBlocks = [];

    mediator.on(ProcessMediator.EVENTS.TRANSACTION, (tx) => {
      receivedTransactions.push(tx);
    });

    mediator.on(ProcessMediator.EVENTS.MERKLE_BLOCK, (merkleBlock) => {
      receivedBlocks.push(merkleBlock);
    });

    subscribeToNewTransactions(
      mediator,
      bloomFilter,
      testTransactionsAgainstFilter,
      bloomFilterEmitterCollection,
    );

    bloomFilterEmitterCollection.test(transactions[0]);
    bloomFilterEmitterCollection.test(transactions[1]);
    bloomFilterEmitterCollection.test(transactions[2]);

    bloomFilterEmitterCollection.emit('block', blocks[0]);

    bloomFilterEmitterCollection.test(transactions[3]);
    bloomFilterEmitterCollection.test(transactions[4]);

    bloomFilterEmitterCollection.emit('block', blocks[1]);

    mediator.emit(ProcessMediator.EVENTS.HISTORICAL_BLOCK_SENT, blocks[0].hash);

    mediator.emit(ProcessMediator.EVENTS.HISTORICAL_DATA_SENT);
    mediator.emit(ProcessMediator.EVENTS.CLIENT_DISCONNECTED);

    expect(receivedTransactions).to.deep.equal([
      transactions[3],
    ]);

    const expectedMerkleBlock = MerkleBlock.build(
      blocks[1].header,
      [
        Buffer.from(transactions[3].hash, 'hex'),
        Buffer.from(transactions[4].hash, 'hex'),
      ],
      [true, false],
    );

    expectedMerkleBlock.hashes = expectedMerkleBlock.hashes
      .map(hash => reverseHash(hash));

    expect(receivedBlocks).to.have.a.lengthOf(1);
    expect(receivedBlocks[0]).to.deep.equal(expectedMerkleBlock);
  });
});
