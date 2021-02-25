const EventEmitter = require('events');
const crypto = require('crypto');

const BlockchainListener = require('../../../../lib/externalApis/tenderdash/BlockchainListener');

describe('BlockchainListener', () => {
  let sinon;
  let wsClientMock;
  let blockchainListener;
  let txQueryMessageMock;
  let transactionHash;
  let blockMessageMock;

  beforeEach(function beforeEach() {
    ({ sinon } = this);
    wsClientMock = new EventEmitter();
    wsClientMock.subscribe = sinon.stub();
    blockchainListener = new BlockchainListener(wsClientMock);
    blockchainListener.start();

    sinon.spy(blockchainListener, 'on');
    sinon.spy(blockchainListener, 'off');
    sinon.spy(blockchainListener, 'emit');

    const txBase64Mock = 'aaaa';
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
              txs: [],
            },
          },
        },
      },
    };
  });

  describe('.getTransactionEventName', () => {
    it('should return event name', () => {
      const topic = BlockchainListener.getTransactionEventName(transactionHash);
      expect(topic).to.be.equal(`transaction:${transactionHash}`);
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

    it('should emit block when new block is arrived', (done) => {
      blockchainListener.on(BlockchainListener.EVENTS.NEW_BLOCK, (message) => {
        expect(message).to.be.deep.equal(blockMessageMock);

        done();
      });

      wsClientMock.emit(BlockchainListener.NEW_BLOCK_QUERY, blockMessageMock);
    });

    it('should emit transaction when transaction is arrived', (done) => {
      const topic = BlockchainListener.getTransactionEventName(transactionHash);

      blockchainListener.on(topic, (message) => {
        expect(message).to.be.deep.equal(txQueryMessageMock);

        done();
      });

      wsClientMock.emit(BlockchainListener.TX_QUERY, txQueryMessageMock);
    });
  });
});
