const DocumentsIndexedTransaction = require('../../../lib/document/DocumentsIndexedTransaction');
const DocumentsDBTransactionIsNotStartedError = require('../../../lib/document/errors/DocumentsDBTransactionIsNotStartedError');
const DocumentsDBTransactionIsAlreadyStartedError = require('../../../lib/document/errors/DocumentsDBTransactionIsAlreadyStartedError');

describe('DocumentsIndexedTransaction', () => {
  let documentsDbTransaction;
  let documentsStoreTransactionMock;
  let documentMongoDBTransactionMock;

  beforeEach(function beforeEach() {
    documentsStoreTransactionMock = {
      start: this.sinon.stub(),
      commit: this.sinon.stub(),
      abort: this.sinon.stub(),
      toObject: this.sinon.stub(),
      populateFromObject: this.sinon.stub(),
    };
    documentMongoDBTransactionMock = {
      start: this.sinon.stub(),
      commit: this.sinon.stub(),
      abort: this.sinon.stub(),
    };

    documentsDbTransaction = new DocumentsIndexedTransaction(
      documentsStoreTransactionMock,
      documentMongoDBTransactionMock,
    );
  });

  describe('#getStoreTransaction', () => {
    it('should return storeTransaction', () => {
      const documentsStoreTransaction = documentsDbTransaction.getStoreTransaction();

      expect(documentsStoreTransaction).to.be.equal(documentsStoreTransactionMock);
    });
  });

  describe('#getMongoDbTransaction', () => {
    it('should return mongoDbTransaction', () => {
      const documentMongoDBTransaction = documentsDbTransaction.getMongoDbTransaction();

      expect(documentMongoDBTransaction).to.be.equal(documentMongoDBTransactionMock);
    });
  });

  describe('#start', () => {
    it('should start transaction', async () => {
      expect(documentsDbTransaction.transactionIsStarted).to.be.false();

      await documentsDbTransaction.start();

      expect(documentsDbTransaction.transactionIsStarted).to.be.true();
      expect(documentsStoreTransactionMock.start).to.be.calledOnce();
      expect(documentMongoDBTransactionMock.start).to.be.calledOnce();
    });

    it('should throw DocumentsDBTransactionIsAlreadyStartedError if transaction is already started', async () => {
      await documentsDbTransaction.start();

      try {
        await documentsDbTransaction.start();

        expect.fail('Should throw an DocumentsDBTransactionIsAlreadyStartedError error');
      } catch (e) {
        expect(e).to.be.instanceOf(DocumentsDBTransactionIsAlreadyStartedError);
      }
    });
  });

  describe('#commit', () => {
    it('should commit transaction', async () => {
      await documentsDbTransaction.start();

      await documentsDbTransaction.commit();

      expect(documentsDbTransaction.transactionIsStarted).to.be.false();
      expect(documentsStoreTransactionMock.commit).to.be.calledOnce();
      expect(documentMongoDBTransactionMock.commit).to.be.calledOnce();
    });

    it('should throw DocumentsDBTransactionIsNotStartedError if transaction is not started', async () => {
      try {
        await documentsDbTransaction.commit();

        expect.fail('Should throw an DocumentsDBTransactionIsNotStartedError error');
      } catch (e) {
        expect(e).to.be.instanceOf(DocumentsDBTransactionIsNotStartedError);
      }
    });
  });

  describe('#abort', () => {
    it('should abort transaction', async () => {
      await documentsDbTransaction.start();
      await documentsDbTransaction.abort();

      expect(documentsDbTransaction.transactionIsStarted).to.be.false();
      expect(documentsStoreTransactionMock.abort).to.be.calledOnce();
      expect(documentMongoDBTransactionMock.abort).to.be.calledOnce();
    });

    it('should throw DocumentsDBTransactionIsNotStartedError if transaction is not started', async () => {
      try {
        await documentsDbTransaction.abort();

        expect.fail('Should throw an DocumentsDBTransactionIsNotStartedError error');
      } catch (e) {
        expect(e).to.be.instanceOf(DocumentsDBTransactionIsNotStartedError);
      }
    });
  });

  describe('#isStarted', () => {
    it('should return if transaction is started or not', () => {
      documentsDbTransaction.transactionIsStarted = false;

      let isStarted = documentsDbTransaction.isStarted();

      expect(isStarted).to.be.false();

      documentsDbTransaction.transactionIsStarted = true;

      isStarted = documentsDbTransaction.isStarted();

      expect(isStarted).to.be.true();
    });
  });

  describe('#toObject', () => {
    it('should return transaction as plain object', () => {
      const plainObject = {
        data: 'soma data',
      };

      documentsStoreTransactionMock.toObject.returns(plainObject);

      const result = documentsDbTransaction.toObject();

      expect(result).to.deep.equal(plainObject);
    });
  });

  describe('#populateFromObject', () => {
    it('should populate transaction using plain object', async () => {
      const plainObject = {
        data: 'soma data',
      };

      await documentsDbTransaction.start();

      documentsDbTransaction.populateFromObject(plainObject);

      expect(documentsStoreTransactionMock.populateFromObject).to.be.calledOnceWithExactly(
        plainObject,
      );
    });
  });
});
