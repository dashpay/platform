const EventEmitter = require('events');
const BlockchainListener = require('../../../../lib/externalApis/tenderdash/BlockchainListener');
const TransactionWaitPeriodExceededError = require('../../../../lib/errors/TransactionWaitPeriodExceededError');

describe('BlockchainListener', () => {
  let sinon;
  let wsClientMock;
  let blockchainListener;
  let txDataMock;

  beforeEach(function beforeEach() {
    ({ sinon } = this);
    wsClientMock = new EventEmitter();
    wsClientMock.subscribe = sinon.stub();
    blockchainListener = new BlockchainListener(wsClientMock);
    blockchainListener.start();

    txDataMock = {
      events: {
        'tx.hash': ['123'],
      },
    };

    sinon.spy(blockchainListener, 'on');
    sinon.spy(blockchainListener, 'off');
    sinon.spy(blockchainListener, 'emit');
  });

  describe('#start', () => {
    it('should subscribe to transaction events from WS client', () => {
      expect(wsClientMock.subscribe).to.be.calledTwice();
      expect(wsClientMock.subscribe.firstCall).to.be.calledWithExactly(
        BlockchainListener.TX_QUERY,
      );
      expect(wsClientMock.subscribe.secondCall).to.be.calledWithExactly(
        BlockchainListener.NEW_BLOCK_QUERY,
      );
    });
  });

  describe('.getTransactionEventName', () => {
    it('should return transaction event name', () => {
      const eventName = BlockchainListener.getTransactionEventName('123');
      expect(eventName).to.be.equal('transaction:123');
    });
  });

  describe('#waitForTransaction', () => {
    it('should remove listener after transaction resolves', async () => {
      const eventName = BlockchainListener.getTransactionEventName('123');
      const txPromise = blockchainListener.waitForTransaction('123', 2000);

      expect(blockchainListener.listenerCount(eventName)).to.be.equal(1);

      setTimeout(() => {
        wsClientMock.emit(BlockchainListener.TX_QUERY, Object.assign({}, txDataMock));
      }, 10);

      const txData = await txPromise;

      // Check that event listener was properly attached
      expect(blockchainListener.on).to.be.calledOnce();
      // Check that transaction data was emitted
      expect(blockchainListener.emit).to.be.calledOnce();
      // Check that the event listener was properly removed
      expect(blockchainListener.off).to.be.calledOnce();
      expect(blockchainListener.listenerCount(eventName)).to.be.equal(0);

      expect(txData).to.be.deep.equal(txDataMock);
    });

    it('should not emit transaction event if event data has no transaction', async () => {
      const eventName = BlockchainListener.getTransactionEventName('123');
      txDataMock = {};

      let error;
      try {
        const txPromise = blockchainListener.waitForTransaction('123', 1000);

        expect(blockchainListener.listenerCount(eventName)).to.be.equal(1);

        setTimeout(() => {
          wsClientMock.emit(BlockchainListener.TX_QUERY, Object.assign({}, txDataMock));
        }, 10);

        await txPromise;
      } catch (e) {
        error = e;
      }

      // Check that the error is correct
      expect(error).to.be.instanceOf(TransactionWaitPeriodExceededError);
      expect(error.message).to.be.equal('Transaction waiting period for 123 exceeded');
      expect(error.getTransactionHash()).to.be.equal('123');

      // Check that event listener was properly attached
      expect(blockchainListener.on).to.be.calledOnce();
      // Check that event listener was properly removed
      expect(blockchainListener.off).to.be.calledOnce();
      expect(blockchainListener.listenerCount(eventName)).to.be.equal(0);
      // Check that no transaction data was emitted
      expect(blockchainListener.emit).to.not.be.called();
    });

    it('should remove listener after timeout has been exceeded', async () => {
      const eventName = BlockchainListener.getTransactionEventName('123');
      let error;
      try {
        await blockchainListener.waitForTransaction('123', 100);
      } catch (e) {
        error = e;
      }

      // Check that the error is correct
      expect(error).to.be.instanceOf(TransactionWaitPeriodExceededError);
      expect(error.message).to.be.equal('Transaction waiting period for 123 exceeded');
      expect(error.getTransactionHash()).to.be.equal('123');

      // Check that event listener was properly attached
      expect(blockchainListener.on).to.be.calledOnce();
      // Check that event listener was properly removed
      expect(blockchainListener.off).to.be.calledOnce();
      expect(blockchainListener.listenerCount(eventName)).to.be.equal(0);
      // Check that no transaction data was emitted
      expect(blockchainListener.emit).to.not.be.called();
    });
  });

  describe('#waitForBlocks', () => {
    it('should wait for n blocks and remove listeners afterwards', async () => {
      const newBlockEvent = BlockchainListener.events.NEW_BLOCK;
      const blockPromise = blockchainListener.waitForBlocks(2);

      expect(blockchainListener.listenerCount(newBlockEvent)).to.be.equal(1);

      setTimeout(() => {
        wsClientMock.emit(BlockchainListener.NEW_BLOCK_QUERY, Object.assign({}, txDataMock));
      }, 10);
      setTimeout(() => {
        wsClientMock.emit(BlockchainListener.NEW_BLOCK_QUERY, Object.assign({}, txDataMock));
      }, 10);

      await blockPromise;

      // Check that event listener was properly attached
      expect(blockchainListener.on).to.be.calledTwice();
      // Check that transaction data was emitted
      expect(blockchainListener.emit).to.be.calledTwice();
      // Check that the event listener was properly removed
      expect(blockchainListener.listenerCount(newBlockEvent)).to.be.equal(0);
    });
  });
});
