const waitForTransactionToBeProvableFactory = require('../../../../../lib/externalApis/tenderdash/waitForTransactionToBeProvable/waitForTransactionToBeProvableFactory');

const TransactionOkResult = require('../../../../../lib/externalApis/tenderdash/waitForTransactionToBeProvable/transactionResult/TransactionOkResult');
const TransactionErrorResult = require('../../../../../lib/externalApis/tenderdash/waitForTransactionToBeProvable/transactionResult/TransactionErrorResult');
const TransactionWaitPeriodExceededError = require('../../../../../lib/errors/TransactionWaitPeriodExceededError');

describe('waitForTransactionToBeProvableFactory', () => {
  let waitForTransactionToBeProvable;
  let waitForTransactionResultMock;
  let waitForTransactionResultResponse;
  let getExistingTransactionResultMock;
  let blockchainListenerMock;
  let hashString;
  let timeout;
  let height;
  let okResult;
  let errorResult;
  let transactionNotFoundError;

  beforeEach(function beforeEach() {
    blockchainListenerMock = { };
    hashString = 'abc';
    timeout = 60000;
    height = 100;

    getExistingTransactionResultMock = this.sinon.stub();

    waitForTransactionResultResponse = {
      promise: null,
      detach: this.sinon.stub(),
    };

    waitForTransactionResultMock = this.sinon.stub().returns(
      waitForTransactionResultResponse,
    );

    waitForTransactionToBeProvable = waitForTransactionToBeProvableFactory(
      waitForTransactionResultMock,
      getExistingTransactionResultMock,
    );

    okResult = new TransactionOkResult({}, height, Buffer.alloc(0));
    errorResult = new TransactionErrorResult({}, height, Buffer.alloc(0));

    transactionNotFoundError = new Error();

    transactionNotFoundError.code = -32603;
    transactionNotFoundError.data = `tx (${hashString}) not found, err: %!w(<nil>)`;
  });

  it('should return existing transaction ok result when next block arrived', async () => {
    getExistingTransactionResultMock.resolves(okResult);

    waitForTransactionResultResponse.promise = new Promise(() => {});

    const actualResult = await waitForTransactionToBeProvable(
      blockchainListenerMock,
      hashString,
      timeout,
    );

    expect(actualResult).to.equal(okResult);

    expect(getExistingTransactionResultMock).to.be.calledOnceWithExactly(
      hashString,
    );

    expect(waitForTransactionResultMock).to.be.calledOnceWith(
      blockchainListenerMock,
      hashString,
    );

    expect(waitForTransactionResultResponse.detach).to.be.called();
  });

  it('should return existing transaction error result', async () => {
    getExistingTransactionResultMock.resolves(errorResult);

    waitForTransactionResultResponse.promise = new Promise(() => {});

    const actualResult = await waitForTransactionToBeProvable(
      blockchainListenerMock,
      hashString,
      timeout,
    );

    expect(actualResult).to.equal(errorResult);

    expect(getExistingTransactionResultMock).to.be.calledOnceWithExactly(
      hashString,
    );

    expect(waitForTransactionResultMock).to.be.calledOnceWith(
      blockchainListenerMock,
      hashString,
    );

    expect(waitForTransactionResultResponse.detach).to.be.called();
  });

  it('should return upcoming transaction ok result', async () => {
    getExistingTransactionResultMock.rejects(transactionNotFoundError);

    waitForTransactionResultResponse.promise = Promise.resolve(okResult);

    const actualResult = await waitForTransactionToBeProvable(
      blockchainListenerMock,
      hashString,
      timeout,
    );

    expect(actualResult).to.equal(okResult);

    expect(getExistingTransactionResultMock).to.be.calledOnceWithExactly(
      hashString,
    );

    expect(waitForTransactionResultMock).to.be.calledOnceWith(
      blockchainListenerMock,
      hashString,
    );

    expect(waitForTransactionResultResponse.detach).to.not.be.called();
  });

  it('should return upcoming transaction error result', async () => {
    getExistingTransactionResultMock.rejects(transactionNotFoundError);

    waitForTransactionResultResponse.promise = Promise.resolve(errorResult);

    const actualResult = await waitForTransactionToBeProvable(
      blockchainListenerMock,
      hashString,
      timeout,
    );

    expect(actualResult).to.equal(errorResult);

    expect(getExistingTransactionResultMock).to.be.calledOnceWithExactly(
      hashString,
    );

    expect(waitForTransactionResultMock).to.be.calledOnceWith(
      blockchainListenerMock,
      hashString,
    );

    expect(waitForTransactionResultResponse.detach).to.not.be.called();
  });

  it('should throw TransactionWaitPeriodExceededError on timeout', async () => {
    timeout = 5;

    getExistingTransactionResultMock.rejects(transactionNotFoundError);

    waitForTransactionResultResponse.promise = new Promise(() => {});

    try {
      await waitForTransactionToBeProvable(
        blockchainListenerMock,
        hashString,
        timeout,
      );

      expect.fail('should throw TransactionWaitPeriodExceededError');
    } catch (e) {
      expect(e).to.be.instanceOf(TransactionWaitPeriodExceededError);

      expect(e.getTransactionHash()).to.equal(hashString);

      expect(getExistingTransactionResultMock).to.be.calledOnceWithExactly(
        hashString,
      );

      expect(waitForTransactionResultMock).to.be.calledOnceWith(
        blockchainListenerMock,
        hashString,
      );

      expect(waitForTransactionResultResponse.detach).to.be.calledOnce();
    }
  });
});
