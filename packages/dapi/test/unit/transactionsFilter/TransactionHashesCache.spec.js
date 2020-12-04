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

    it('should add transaction to instant send waiting list', () => {
      const [firstTx] = transactions;

      transactionHashesCache.addTransaction(firstTx);

      expect(transactionHashesCache.blocksProcessed).to.be.equal(0);
      expect(transactionHashesCache.transactionHashesMap).to.deep.equal({
        [firstTx.hash]: 0,
      });
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

    it('should increment blocks count on every transaction in the cache', () => {
      const [firstTx, secondTx] = transactions;

      transactionHashesCache.addTransaction(firstTx);

      expect(transactionHashesCache.blocksProcessed).to.be.equal(0);
      expect(transactionHashesCache.transactionHashesMap).to.deep.equal(
        {
          [firstTx.hash]: 0,
        },
      );

      transactionHashesCache.addBlock(blocks[0]);
      transactionHashesCache.addTransaction(secondTx);
      transactionHashesCache.addBlock(blocks[1]);

      expect(transactionHashesCache.blocksProcessed).to.be.equal(2);
      expect(transactionHashesCache.transactionHashesMap).to.deep.equal({
        [firstTx.hash]: 0,
        [secondTx.hash]: 1,
      });
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

  describe('#isInInstantLockCache', () => {
    it('should return true if the transaction in the cache', () => {
      const [firstTx] = transactions;

      transactionHashesCache.addTransaction(firstTx);

      expect(transactionHashesCache.isInInstantLockCache(firstTx.hash)).to.be.true();
    });

    it('should return false if transaction is not in cache', () => {
      const [firstTx, secondTx] = transactions;

      transactionHashesCache.addTransaction(firstTx);

      expect(transactionHashesCache.isInInstantLockCache(secondTx.hash)).to.be.false();
    });
  });

  describe('#removeTransactionHashFromInstantSendLockWaitingList', () => {
    it('should remove transaction from a waiting list', () => {
      const [firstTx, secondTx] = transactions;

      transactionHashesCache.addTransaction(firstTx);
      transactionHashesCache.addTransaction(secondTx);

      expect(transactionHashesCache.transactionHashesMap).to.be.deep.equal({
        [firstTx.hash]: 0,
        [secondTx.hash]: 0,
      });

      transactionHashesCache.removeTransactionHashFromInstantSendLockWaitingList(firstTx.hash);

      expect(transactionHashesCache.transactionHashesMap).to.be.deep.equal({
        [secondTx.hash]: 0,
      });
    });
  });
});
