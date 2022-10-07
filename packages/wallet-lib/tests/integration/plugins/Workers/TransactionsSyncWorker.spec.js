describe('TransactionsSyncWorker', () => {
  describe('Basic synchronization', () => {
    context('Historical sync', () => {
      it('should process transactions and a merkle block');

      it('should handle addresses gap filling');

      it('should handle merkle block rejection');
    });

    context('Continuous sync', () => {
      context('2 TXs in the same block', () => {
        it('should process unconfirmed transaction');
        it('should handle addresses gap filling');
        it('should confirm transactions with a merkle block');
      });
      context('2 TXs in 2 blocks', () => {
        it('should process two unconfirmed transactions');
        it('should process first TX in the first merkle block');
        it('should process second TX in the second merkle block');
      });
    });
  });

  context('Synchronization with storage adapter', () => {
    context('First launch', () => {
      it('should start historical sync and stop in the middle');
    });

    context('Second launch', () => {
      it('should finish historical sync after restart');
      it('should proceed with the continuous sync');
    });

    context('Third launch', () => {
      it('should sync up to the latest chain height after restart');
      it('should proceed with the continuous sync');
    });
  });
});
