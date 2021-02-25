const EventEmitter = require('events');

const waitForTransactionResult = require('../../../../../lib/externalApis/tenderdash/waitForTransactionToBeProvable/waitForTransactionResult');

const BlockchainListener = require('../../../../../lib/externalApis/tenderdash/BlockchainListener');

const TransactionOkResult = require('../../../../../lib/externalApis/tenderdash/waitForTransactionToBeProvable/transactionResult/TransactionOkResult');
const TransactionErrorResult = require('../../../../../lib/externalApis/tenderdash/waitForTransactionToBeProvable/transactionResult/TransactionErrorResult');

describe('waitForTransactionResult', () => {
  let blockchainListenerMock;
  let hashString;
  let topic;
  let tx;
  let height;

  beforeEach(function beforeEach() {
    blockchainListenerMock = new EventEmitter();

    this.sinon.spy(blockchainListenerMock);

    hashString = 'abc';

    topic = BlockchainListener.getTransactionEventName(hashString);

    tx = 'aGVsbG8h';

    height = 100;
  });

  it('should resolve TransactionOkResult when transaction result is emitted', async () => {
    const { promise } = waitForTransactionResult(blockchainListenerMock, hashString);

    const result = {
      code: 0,
    };

    const data = { data: { value: { TxResult: { result, tx, height } } } };

    blockchainListenerMock.emit(topic, data);

    const transactionResult = await promise;

    expect(transactionResult).to.be.instanceOf(TransactionOkResult);
    expect(transactionResult.getResult()).to.equal(result);
    expect(transactionResult.getHeight()).to.equal(100);
    expect(transactionResult.getTransaction()).to.deep.equal(Buffer.from(tx, 'base64'));

    expect(blockchainListenerMock.off).to.be.calledOnceWith(topic);
  });

  it('should resolve TransactionErrorResult when transaction result is emitted', async () => {
    const { promise } = waitForTransactionResult(blockchainListenerMock, hashString);

    const result = {
      code: 1,
    };

    const data = { data: { value: { TxResult: { result, tx, height } } } };

    blockchainListenerMock.emit(topic, data);

    const transactionResult = await promise;

    expect(transactionResult).to.be.instanceOf(TransactionErrorResult);
    expect(transactionResult.getResult()).to.equal(result);
    expect(transactionResult.getHeight()).to.equal(100);
    expect(transactionResult.getTransaction()).to.deep.equal(Buffer.from(tx, 'base64'));

    expect(blockchainListenerMock.off).to.be.calledOnceWith(topic);
  });

  it('should remove listeners on detach', () => {
    const { detach } = waitForTransactionResult(blockchainListenerMock, hashString);

    detach();

    expect(blockchainListenerMock.off).to.be.calledOnceWith(topic);
  });
});
