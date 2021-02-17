const EventEmitter = require('events');

const waitForTransactionCommitment = require('../../../../../../lib/externalApis/tenderdash/blockchainListener/waitForTransactionToBeProvable/waitForTransactionCommitment');

const BlockchainListener = require('../../../../../../lib/externalApis/tenderdash/blockchainListener/BlockchainListener');

describe('waitForTransactionCommitment', () => {
  let blockchainListenerMock;
  let hashString;
  let txInBlockTopic;

  beforeEach(function beforeEach() {
    blockchainListenerMock = new EventEmitter();

    this.sinon.spy(blockchainListenerMock);

    hashString = 'abc';

    txInBlockTopic = BlockchainListener
      .getTransactionAddedToTheBlockEventName(hashString.toLowerCase());
  });

  it('should resolve promise after block with transaction and one more', async () => {
    const { promise } = waitForTransactionCommitment(blockchainListenerMock, hashString);

    // Emit new block
    blockchainListenerMock.emit(BlockchainListener.EVENTS.NEW_BLOCK);

    // Assert promise is pending
    expect(await Promise.race([promise, true])).to.equal(true);

    // Emit new block with transaction
    blockchainListenerMock.emit(txInBlockTopic);

    // Assert promise is pending
    expect(await Promise.race([promise, true])).to.equal(true);

    // Emit next block
    blockchainListenerMock.emit(BlockchainListener.EVENTS.NEW_BLOCK);

    const result = await promise;

    expect(result).to.be.undefined();

    expect(blockchainListenerMock.off).to.be.calledOnceWith(BlockchainListener.EVENTS.NEW_BLOCK);
    expect(blockchainListenerMock.once).to.be.calledOnceWith(txInBlockTopic);
  });

  it('should remove listeners on detach', () => {
    const { detach } = waitForTransactionCommitment(blockchainListenerMock, hashString);

    detach();

    expect(blockchainListenerMock.off).to.be.calledTwice();
    expect(blockchainListenerMock.off.withArgs(txInBlockTopic)).to.be.called();
    expect(blockchainListenerMock.off.withArgs(BlockchainListener.EVENTS.NEW_BLOCK)).to.be.called();
  });
});
