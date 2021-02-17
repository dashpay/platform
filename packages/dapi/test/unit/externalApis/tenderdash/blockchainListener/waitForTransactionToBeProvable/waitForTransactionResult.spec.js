const EventEmitter = require('events');

const waitForTransactionResult = require('../../../../../../lib/externalApis/tenderdash/blockchainListener/waitForTransactionToBeProvable/waitForTransactionResult');

const BlockchainListener = require('../../../../../../lib/externalApis/tenderdash/blockchainListener/BlockchainListener');

const TransactionOkResult = require('../../../../../../lib/externalApis/tenderdash/blockchainListener/waitForTransactionToBeProvable/transactionResult/TransactionOkResult');
const TransactionErrorResult = require('../../../../../../lib/externalApis/tenderdash/blockchainListener/waitForTransactionToBeProvable/transactionResult/TransactionErrorResult');

describe('waitForTransactionResult', () => {
  let blockchainListenerMock;
  let hashString;
  let topic;
  let tx;

  beforeEach(function beforeEach() {
    blockchainListenerMock = new EventEmitter();

    this.sinon.spy(blockchainListenerMock);

    hashString = 'abc';

    topic = BlockchainListener.getTransactionEventName(hashString);

    tx = 'aGVsbG8h';
  });

  it('should resolve promise with TransactionOkResult when transaction result is emitted', async () => {
    const { promise } = waitForTransactionResult(blockchainListenerMock, hashString);

    const result = {
      code: 0,
    };

    const data = { data: { value: { TxResult: { result, tx } } } };

    blockchainListenerMock.emit(topic, data);

    const transactionResult = await promise;

    expect(transactionResult).to.be.instanceOf(TransactionOkResult);
    expect(transactionResult.getDeliverResult()).to.equal(result);
    expect(transactionResult.getTransaction()).to.deep.equal(Buffer.from(tx, 'base64'));

    expect(blockchainListenerMock.off).to.be.calledOnceWith(topic);
  });

  it('should reject promise with TransactionErrorResult when transaction result is emitted', async () => {
    const { promise } = waitForTransactionResult(blockchainListenerMock, hashString);

    const result = {
      code: 1,
    };

    const data = { data: { value: { TxResult: { result, tx } } } };

    blockchainListenerMock.emit(topic, data);

    try {
      await promise;

      expect.fail('should throw TransactionErrorResult');
    } catch (transactionResult) {
      expect(transactionResult).to.be.instanceOf(TransactionErrorResult);
      expect(transactionResult.getDeliverResult()).to.equal(result);
      expect(transactionResult.getTransaction()).to.deep.equal(Buffer.from(tx, 'base64'));
    }

    expect(blockchainListenerMock.off).to.be.calledOnceWith(topic);
  });

  it('should remove listeners on detach', () => {
    const { detach } = waitForTransactionResult(blockchainListenerMock, hashString);

    detach();

    expect(blockchainListenerMock.off).to.be.calledOnceWith(topic);
  });
});
