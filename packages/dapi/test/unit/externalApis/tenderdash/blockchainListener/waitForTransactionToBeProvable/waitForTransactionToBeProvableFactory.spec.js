const waitForTransactionToBeProvableFactory = require('../../../../../../lib/externalApis/tenderdash/blockchainListener/waitForTransactionToBeProvable/waitForTransactionToBeProvableFactory');

const TransactionOkResult = require('../../../../../../lib/externalApis/tenderdash/blockchainListener/waitForTransactionToBeProvable/transactionResult/TransactionOkResult');
const TransactionErrorResult = require('../../../../../../lib/externalApis/tenderdash/blockchainListener/waitForTransactionToBeProvable/transactionResult/TransactionErrorResult');
const TransactionWaitPeriodExceededError = require('../../../../../../lib/errors/TransactionWaitPeriodExceededError');

describe('waitForTransactionToBeProvableFactory', () => {
  let waitForTransactionToBeProvable;
  let waitForTransactionResultMock;
  let waitForTransactionResultResponse;
  let waitForTransactionCommitmentMock;
  let waitForTransactionCommitmentResponse;
  let blockchainListenerMock;
  let hashString;
  let timeout;

  beforeEach(function beforeEach() {
    blockchainListenerMock = { };
    hashString = 'abc';
    timeout = 60000;

    waitForTransactionResultResponse = {
      promise: null,
      detach: this.sinon.stub(),
    };

    waitForTransactionResultMock = this.sinon.stub().returns(
      waitForTransactionResultResponse,
    );

    waitForTransactionCommitmentResponse = {
      promise: null,
      detach: this.sinon.stub(),
    };

    waitForTransactionCommitmentMock = this.sinon.stub().returns(
      waitForTransactionCommitmentResponse,
    );

    waitForTransactionToBeProvable = waitForTransactionToBeProvableFactory(
      waitForTransactionResultMock,
      waitForTransactionCommitmentMock,
    );
  });

  it('should return TransactionOkResult and wait for proofs', async () => {
    const expectedResult = new TransactionOkResult({}, Buffer.alloc(0));

    waitForTransactionResultResponse.promise = Promise.resolve(expectedResult);

    waitForTransactionCommitmentResponse.promise = Promise.resolve();

    const actualResult = await waitForTransactionToBeProvable(
      blockchainListenerMock,
      hashString,
      timeout,
    );

    expect(actualResult).to.equal(expectedResult);

    expect(waitForTransactionResultMock).to.be.calledOnceWithExactly(
      blockchainListenerMock,
      hashString,
    );

    expect(waitForTransactionCommitmentMock).to.be.calledOnceWithExactly(
      blockchainListenerMock,
      hashString,
    );

    expect(waitForTransactionResultResponse.detach).to.not.be.called();
    expect(waitForTransactionCommitmentResponse.detach).to.not.be.called();
  });

  it('should return TransactionErrorResult', async () => {
    const expectedResult = new TransactionErrorResult({}, Buffer.alloc(0));

    waitForTransactionResultResponse.promise = Promise.reject(expectedResult);

    waitForTransactionCommitmentResponse.promise = new Promise(() => {});

    const actualResult = await waitForTransactionToBeProvable(
      blockchainListenerMock,
      hashString,
      timeout,
    );

    expect(actualResult).to.equal(expectedResult);

    expect(waitForTransactionResultMock).to.be.calledOnceWithExactly(
      blockchainListenerMock,
      hashString,
    );

    expect(waitForTransactionCommitmentMock).to.be.calledOnceWithExactly(
      blockchainListenerMock,
      hashString,
    );

    expect(waitForTransactionResultResponse.detach).to.not.be.called();
    expect(waitForTransactionCommitmentResponse.detach).to.not.be.called();
  });

  it('should throw TransactionWaitPeriodExceededError', async () => {
    timeout = 5;

    waitForTransactionResultResponse.promise = new Promise(() => {});

    waitForTransactionCommitmentResponse.promise = new Promise(() => {});

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

      expect(waitForTransactionResultMock).to.be.calledOnceWithExactly(
        blockchainListenerMock,
        hashString,
      );

      expect(waitForTransactionCommitmentMock).to.be.calledOnceWithExactly(
        blockchainListenerMock,
        hashString,
      );

      expect(waitForTransactionResultResponse.detach).to.be.calledOnce();
      expect(waitForTransactionCommitmentResponse.detach).to.be.calledOnce();
    }
  });
});
