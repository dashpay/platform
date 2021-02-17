const EventEmitter = require('events');
const crypto = require('crypto');
const BlockchainListener = require('../../../../lib/externalApis/tenderdash/blockchainListener/BlockchainListener');

describe('BlockchainListener', () => {
  let sinon;
  let wsClientMock;
  let blockchainListener;
  let txQueryMessageMock;
  let blockMessageMock;
  let transactionHash;
  let txBase64Mock;
  let txBufferMock;
  let emptyBlockMessage;

  beforeEach(function beforeEach() {
    ({ sinon } = this);
    wsClientMock = new EventEmitter();
    wsClientMock.subscribe = sinon.stub();
    blockchainListener = new BlockchainListener(wsClientMock);
    blockchainListener.start();

    sinon.spy(blockchainListener, 'on');
    sinon.spy(blockchainListener, 'off');
    sinon.spy(blockchainListener, 'emit');

    txBase64Mock = 'aaaa';
    txBufferMock = Buffer.from(txBase64Mock, 'base64');
    transactionHash = crypto.createHash('sha256')
      .update(Buffer.from(txBase64Mock, 'base64'))
      .digest()
      .toString('hex');

    txQueryMessageMock = {
      events: {
        'tx.hash': [transactionHash],
      },
    };

    blockMessageMock = {
      data: {
        value: {
          block: {
            data: {
              txs: [txBase64Mock],
            },
          },
        },
      },
    };

    emptyBlockMessage = {
      data: {
        value: {
          block: {
            data: {
              txs: [],
            },
          },
        },
      },
    };
  });

  describe('.getTransactionEventName', () => {
    it('should work', () => {
      const topic = BlockchainListener.getTransactionEventName(transactionHash);
      expect(topic).to.be.equal(`transaction:${transactionHash}`);
    });
  });

  describe('.getTransactionAddedToTheBlockEventName', () => {
    it('should work', () => {
      const topic = BlockchainListener.getTransactionAddedToTheBlockEventName(transactionHash);
      expect(topic).to.be.equal(`blockTransactionAdded:${transactionHash}`);
    });
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

    it('should emit transaction hash when transaction is added to the block', (done) => {
      const topic = BlockchainListener.getTransactionEventName(transactionHash);
      blockchainListener.on(topic, (message) => {
        expect(message).to.be.deep.equal(txQueryMessageMock);
        done();
      });

      wsClientMock.emit(BlockchainListener.TX_QUERY, txQueryMessageMock);
    });

    it('should emit transaction buffer when received a block with this tx from WS connection', (done) => {
      const topic = BlockchainListener.getTransactionAddedToTheBlockEventName(transactionHash);
      blockchainListener.on(topic, (transactionBuffer) => {
        expect(transactionBuffer).to.be.deep.equal(txBufferMock);
        done();
      });

      wsClientMock.emit(BlockchainListener.NEW_BLOCK_QUERY, blockMessageMock);
    });

    it('should not emit any transaction hashes if block contents are empty', (done) => {
      wsClientMock.emit(BlockchainListener.NEW_BLOCK_QUERY, emptyBlockMessage);

      setTimeout(() => {
        expect(blockchainListener.on).to.not.be.called();
        done();
      }, 100);
    });
  });
});
