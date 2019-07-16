const { MerkleBlock } = require('@dashevo/dashcore-lib');

const sinon = require('sinon');
const { expect } = require('chai');

const TransactionHashesCache = require('../../../lib/transactionsFilter/TransactionHashesCache');

describe('TransactionHashesCache', () => {
  let transactions;
  let blocks;
  let merkleBlocks;
  let transactionHashesCache;

  beforeEach(() => {
    transactions = [
      { hash: '000000000000000000000000000000000000000000000000000000000000001b' },
      { hash: '000000000000000000000000000000000000000000000000000000000000002b' },
      { hash: '000000000000000000000000000000000000000000000000000000000000003b' },
      { hash: '000000000000000000000000000000000000000000000000000000000000004b' },
      { hash: '000000000000000000000000000000000000000000000000000000000000005b' },
      { hash: '000000000000000000000000000000000000000000000000000000000000006b' },
    ];

    blocks = [
      {
        hash: '000000000000000000000000000000000000000000000000000000000000001b',
        transactions: [transactions[0], transactions[1]],
        header: {
          hash: '000000000000000000000000000000000000000000000000000000000000001b',
        },
      },
      {
        hash: '000000000000000000000000000000000000000000000000000000000000002b',
        transactions: [transactions[2], transactions[3]],
        header: {
          hash: '000000000000000000000000000000000000000000000000000000000000002b',
        },
      },
      {
        hash: '000000000000000000000000000000000000000000000000000000000000003b',
        transactions: [transactions[4], transactions[5]],
        header: {
          hash: '000000000000000000000000000000000000000000000000000000000000003b',
        },
      },

      {
        hash: '000000000000000000000000000000000000000000000000000000000000004b',
        transactions: [transactions[0], transactions[1]],
        header: {
          hash: '000000000000000000000000000000000000000000000000000000000000004b',
        },
      },
    ];

    merkleBlocks = blocks.map(block => ({ header: { hash: block.hash } }));

    sinon.stub(MerkleBlock, 'build');

    blocks.forEach((block, index) => {
      MerkleBlock.build
        .withArgs(block.header, sinon.match.any, sinon.match.any)
        .returns(merkleBlocks[index]);
    });

    transactionHashesCache = new TransactionHashesCache();
  });

  afterEach(() => {
    MerkleBlock.build.restore();
  });

  describe('#addTransaction', () => {
    it('should add transaction', () => {
      const [firstTx] = transactions;

      transactionHashesCache.addTransaction(firstTx);

      expect(transactionHashesCache.transactions).to.deep.equal([
        {
          transaction: firstTx,
          isRetrieved: false,
        },
      ]);
    });
  });

  describe('#addBlock', () => {
    it('should add a block if it has matched transactions', () => {
      const [block] = blocks;

      transactionHashesCache.addTransaction(transactions[0]);
      transactionHashesCache.addTransaction(transactions[1]);

      transactionHashesCache.addBlock(block);

      expect(transactionHashesCache.blocks).to.deep.equal(
        [block],
      );
    });

    it('should remove data if cache size is reached', () => {
      transactionHashesCache.cacheSize = 2;

      transactionHashesCache.addTransaction(transactions[0]);
      transactionHashesCache.addTransaction(transactions[1]);
      transactionHashesCache.addTransaction(transactions[2]);
      transactionHashesCache.addTransaction(transactions[3]);
      transactionHashesCache.addTransaction(transactions[4]);
      transactionHashesCache.addTransaction(transactions[5]);

      transactionHashesCache.addBlock(blocks[0]);
      transactionHashesCache.addBlock(blocks[1]);
      transactionHashesCache.addBlock(blocks[2]);

      expect(transactionHashesCache.transactions).to.deep.equal([
        { transaction: transactions[2], isRetrieved: false },
        { transaction: transactions[3], isRetrieved: false },
        { transaction: transactions[4], isRetrieved: false },
        { transaction: transactions[5], isRetrieved: false },
      ]);

      expect(transactionHashesCache.merkleBlocks).to.deep.equal([
        { merkleBlock: merkleBlocks[1], isRetrieved: false },
        { merkleBlock: merkleBlocks[2], isRetrieved: false },
      ]);

      expect(transactionHashesCache.blocks).to.deep.equal(
        [blocks[1], blocks[2]],
      );
    });
  });

  describe('#getBlockCount', () => {
    it('should return block count', () => {
      transactionHashesCache.addTransaction(transactions[0]);
      transactionHashesCache.addTransaction(transactions[1]);
      transactionHashesCache.addTransaction(transactions[2]);
      transactionHashesCache.addTransaction(transactions[3]);

      transactionHashesCache.addBlock(blocks[0]);
      transactionHashesCache.addBlock(blocks[1]);

      expect(transactionHashesCache.getBlockCount()).to.equal(2);
    });
  });

  describe('#removeDataByBlockHash', () => {
    it('should remove data by block hash', () => {
      transactionHashesCache.addTransaction(transactions[0]);
      transactionHashesCache.addTransaction(transactions[1]);
      transactionHashesCache.addTransaction(transactions[2]);
      transactionHashesCache.addTransaction(transactions[3]);
      transactionHashesCache.addTransaction(transactions[4]);
      transactionHashesCache.addTransaction(transactions[5]);

      transactionHashesCache.addBlock(blocks[0]);
      transactionHashesCache.addBlock(blocks[1]);

      transactionHashesCache.removeDataByBlockHash(blocks[0].hash);

      expect(transactionHashesCache.transactions).to.deep.equal([
        { transaction: transactions[2], isRetrieved: false },
        { transaction: transactions[3], isRetrieved: false },
        { transaction: transactions[4], isRetrieved: false },
        { transaction: transactions[5], isRetrieved: false },
      ]);

      expect(transactionHashesCache.merkleBlocks).to.deep.equal([
        { merkleBlock: merkleBlocks[1], isRetrieved: false },
      ]);

      expect(transactionHashesCache.blocks).to.deep.equal(
        [blocks[1]],
      );
    });
  });

  describe('#getUnretrievedTrasactions', () => {
    it('should return unsent transactions and mark them as sent', () => {
      transactionHashesCache.addTransaction(transactions[0]);
      transactionHashesCache.addTransaction(transactions[1]);

      expect(transactionHashesCache.getUnretrievedTransactions()).to.deep.equal([
        transactions[0],
        transactions[1],
      ]);

      expect(transactionHashesCache.getUnretrievedTransactions()).to.deep.equal([]);
    });
  });

  describe('#getUnretrievedMerkleBlocks', () => {
    it('should return unsent merkle blocks and mark them as sent', () => {
      transactionHashesCache.addTransaction(transactions[0]);
      transactionHashesCache.addTransaction(transactions[1]);
      transactionHashesCache.addTransaction(transactions[2]);
      transactionHashesCache.addTransaction(transactions[3]);

      transactionHashesCache.addBlock(blocks[0]);
      transactionHashesCache.addBlock(blocks[1]);

      expect(transactionHashesCache.getUnretrievedMerkleBlocks()).to.deep.equal([
        merkleBlocks[0],
        merkleBlocks[1],
      ]);

      expect(transactionHashesCache.getUnretrievedMerkleBlocks()).to.deep.equal([]);
    });
  });
});
