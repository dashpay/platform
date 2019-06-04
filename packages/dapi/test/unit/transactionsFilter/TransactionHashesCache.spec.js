const { expect } = require('chai');

const TransactionHashesCache = require('../../../lib/transactionsFilter/TransactionHashesCache');

describe('TransactionHashesCache', () => {
  let transactionHash;
  let transactionHashesCache;
  let blockTransactionHashes;

  beforeEach(() => {
    transactionHash = 'c4970326400177ce67ec582425a698b85ae03cae2b0d168e87eed697f1388e4b';

    blockTransactionHashes = [
      '31cc138ef81802dd836e3aadde8788221a1cf0a8993c77ddd06421b95e6dfc76',
      'e2369e7645f3917101e057f442e4a062f02ca7cc24b535ef5c991c7ed1782fdf',
      'f1dc94df6ee0a70f4a8af12ccf121a22ded76fa3519d6acd9ab97604009541c3',
      'd340e1628d5e6db3f67dc3947a304ce52921ae35b7276ae8da925109b9026b0d',
    ];

    transactionHashesCache = new TransactionHashesCache();
  });

  describe('#add', () => {
    it('should add transaction hash with 0 confirmations', () => {
      transactionHashesCache.add(transactionHash);

      expect(transactionHashesCache.transactionHashes).to.deep.equal({
        [transactionHash]: 0,
      });
    });
  });

  describe('#getConfirmationsCount', () => {
    it('should return confirmations count for a specified transaction', () => {
      const confirmationCount = 3;

      transactionHashesCache.transactionHashes = {
        [transactionHash]: confirmationCount,
      };

      const result = transactionHashesCache.getConfirmationsCount(transactionHash);

      expect(result).to.equal(confirmationCount);
    });
  });

  describe('#updateByBlockTransactionHashes', () => {
    it('should do nothing if there are no cached transactions', () => {
      transactionHashesCache.updateByBlockTransactionHashes(blockTransactionHashes);

      expect(transactionHashesCache.transactionHashes).to.deep.equal({});
    });

    it('should set confirmations count to 1 if a block contains a transaction from the cache', () => {
      const [firstHashFromBlock, secondHashFromBlock] = blockTransactionHashes;

      transactionHashesCache.transactionHashes = {
        [firstHashFromBlock]: 0,
        [secondHashFromBlock]: 0,
        [transactionHash]: 0,
      };

      transactionHashesCache.updateByBlockTransactionHashes(blockTransactionHashes);

      expect(transactionHashesCache.transactionHashes).to.deep.equal({
        [firstHashFromBlock]: 1,
        [secondHashFromBlock]: 1,
        [transactionHash]: 0,
      });
    });

    it('should increment confirmations count by 1 if a block doesn\'t contain a transaction from cache', () => {
      const [firstHashFromBlock, secondHashFromBlock] = blockTransactionHashes;

      transactionHashesCache.transactionHashes = {
        [firstHashFromBlock]: 1,
        [secondHashFromBlock]: 1,
        [transactionHash]: 0,
      };

      const [,, thirdHashFromBlock, fourthHashFromBlock] = blockTransactionHashes;

      transactionHashesCache.updateByBlockTransactionHashes([
        thirdHashFromBlock,
        fourthHashFromBlock,
      ]);

      expect(transactionHashesCache.transactionHashes).to.deep.equal({
        [firstHashFromBlock]: 2,
        [secondHashFromBlock]: 2,
        [transactionHash]: 0,
      });
    });

    it('should remove a transaction from cache if it has more than N confirmations', () => {
      const [firstHashFromBlock, secondHashFromBlock] = blockTransactionHashes;

      transactionHashesCache.transactionHashes = {
        [firstHashFromBlock]: transactionHashesCache.cacheSize,
        [secondHashFromBlock]: transactionHashesCache.cacheSize - 1,
        [transactionHash]: 0,
      };

      const [,, thirdHashFromBlock, fourthHashFromBlock] = blockTransactionHashes;

      transactionHashesCache.updateByBlockTransactionHashes([
        thirdHashFromBlock,
        fourthHashFromBlock,
      ]);

      expect(transactionHashesCache.transactionHashes).to.deep.equal({
        [secondHashFromBlock]: 10,
        [transactionHash]: 0,
      });
    });
  });
});
