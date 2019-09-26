const MongoDBTransaction = require('../../../lib/mongoDb/MongoDBTransaction');
const TransactionIsNotStartedError = require('../../../lib/mongoDb/errors/TransactionIsNotStartedError');
const TransactionIsAlreadyStartedError = require('../../../lib/mongoDb/errors/TransactionIsAlreadyStartedError');

describe('MongoDBTransaction', () => {
  let mongoClientMock;
  let mongoDBTransaction;
  let sessionMock;
  let transactionFunctionMock;

  beforeEach(function beforeEach() {
    sessionMock = {
      startTransaction: this.sinon.stub(),
      commitTransaction: this.sinon.stub(),
      abortTransaction: this.sinon.stub(),
    };

    mongoClientMock = {
      startSession: this.sinon.stub().returns(sessionMock),
    };

    mongoDBTransaction = new MongoDBTransaction(mongoClientMock);
    transactionFunctionMock = this.sinon.stub().resolves(this.sinon.stub());
  });

  describe('#start', () => {
    it('should start transaction', () => {
      mongoDBTransaction.start();

      expect(mongoClientMock.startSession).to.be.calledOnce();
      expect(sessionMock.startTransaction).to.be.calledOnce();
    });

    it('should throw an error, if transaction is already started', () => {
      mongoDBTransaction.start();

      try {
        mongoDBTransaction.start();

        expect.fail('should throw "Transaction is already started" error');
      } catch (error) {
        expect(error).to.be.an.instanceOf(TransactionIsAlreadyStartedError);
        expect(error.message).to.be.equal('Transaction is already started');
        expect(mongoClientMock.startSession).to.be.calledOnce();
        expect(sessionMock.startTransaction).to.be.calledOnce();
      }
    });
  });

  describe('#commit', () => {
    it('should commit transaction', async () => {
      mongoDBTransaction.start();
      await mongoDBTransaction.commit();

      expect(sessionMock.commitTransaction).to.be.calledOnce();
      expect(mongoDBTransaction.isTransactionStarted).to.be.false();
    });

    it('should commit two transactions one after another', async () => {
      mongoDBTransaction.start();
      await mongoDBTransaction.commit();

      mongoDBTransaction.start();
      await mongoDBTransaction.commit();

      expect(sessionMock.commitTransaction).to.be.calledTwice();
      expect(mongoDBTransaction.isTransactionStarted).to.be.false();
    });

    it('should throw an error if transaction is not started', async () => {
      try {
        await mongoDBTransaction.commit();

        expect.fail('should throw "Transaction is not started" error');
      } catch (error) {
        expect(error).to.be.an.instanceOf(TransactionIsNotStartedError);
        expect(error.message).to.be.equal('Transaction is not started');
        expect(sessionMock.commitTransaction).to.have.not.been.called();
      }
    });

    it('should catch UNKNOWN_TRANSACTION_COMMIT_RESULT error', async () => {
      const { ERRORS } = MongoDBTransaction;

      sessionMock.commitTransaction.onFirstCall().throws({
        errorLabels: [ERRORS.UNKNOWN_TRANSACTION_COMMIT_RESULT],
      });

      mongoDBTransaction.start();
      await mongoDBTransaction.commit();

      expect(sessionMock.commitTransaction).to.be.calledTwice();
    });

    it('should throw an error', async () => {
      sessionMock.commitTransaction.throws('UnknownError');

      mongoDBTransaction.start();

      try {
        await mongoDBTransaction.commit();

        expect.fail('should throw "UnknownError"');
      } catch (error) {
        expect(sessionMock.commitTransaction).to.be.calledOnce();
      }
    });
  });

  describe('#abort', () => {
    it('should abort session', async () => {
      mongoDBTransaction.start();
      await mongoDBTransaction.abort();

      expect(sessionMock.abortTransaction).to.be.calledOnce();
      expect(mongoDBTransaction.isTransactionStarted).to.be.false();
    });

    it('should commit new transaction after aborted transaction', async () => {
      mongoDBTransaction.start();
      await mongoDBTransaction.abort();

      mongoDBTransaction.start();
      await mongoDBTransaction.commit();

      expect(sessionMock.abortTransaction).to.be.calledOnce();
      expect(sessionMock.commitTransaction).to.be.calledOnce();
    });

    it('should throw an error if transaction is not started', async () => {
      try {
        await mongoDBTransaction.abort();

        expect.fail('should throw "Transaction is not started" error');
      } catch (error) {
        expect(error).to.be.an.instanceOf(TransactionIsNotStartedError);
        expect(error.message).to.be.equal('Transaction is not started');
        expect(sessionMock.commitTransaction).to.have.not.been.called();
      }
    });
  });

  describe('#runWithTransaction', async () => {
    it('should run function with transaction', async () => {
      mongoDBTransaction.start();
      await mongoDBTransaction.runWithTransaction(transactionFunctionMock);

      expect(transactionFunctionMock).to.be.calledOnce();
    });

    it('should catch TRANSIENT_TRANSACTION_ERROR', async () => {
      const { ERRORS } = MongoDBTransaction;

      transactionFunctionMock.onFirstCall().throws({
        errorLabels: [ERRORS.TRANSIENT_TRANSACTION_ERROR],
      });

      mongoDBTransaction.start();
      await mongoDBTransaction.runWithTransaction(transactionFunctionMock);

      expect(transactionFunctionMock).to.be.calledTwice();
    });

    it('should throw an error', async () => {
      transactionFunctionMock.throws('UnknownError');

      mongoDBTransaction.start();

      try {
        await mongoDBTransaction.runWithTransaction(transactionFunctionMock);

        expect.fail('should throw "UnknownError"');
      } catch (error) {
        expect(transactionFunctionMock).to.be.calledOnce();
      }
    });

    it('should throw an error if transaction is not started', async () => {
      try {
        await mongoDBTransaction.runWithTransaction(transactionFunctionMock);

        expect.fail('should throw "Transaction is not started" error');
      } catch (error) {
        expect(error).to.be.an.instanceOf(TransactionIsNotStartedError);
        expect(error.message).to.be.equal('Transaction is not started');
        expect(transactionFunctionMock).to.have.not.been.called();
      }
    });
  });
});
