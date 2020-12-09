const BlockExecutionStoreTransactions = require('../../../lib/blockExecution/BlockExecutionStoreTransactions');
const BlockExecutionStoreTransactionIsAlreadyStartedError = require('../../../lib/blockExecution/errors/BlockExecutionStoreTransactionIsAlreadyStartedError');
const BlockExecutionStoreTransactionIsNotStartedError = require('../../../lib/blockExecution/errors/BlockExecutionStoreTransactionIsNotStartedError');
const BlockExecutionStoreTransactionIsNotDefinedError = require('../../../lib/blockExecution/errors/BlockExecutionStoreTransactionIsNotDefinedError');

describe('BlockExecutionStoreTransactions', () => {
  let blockExecutionStoreTransactions;
  let commonStoreMock;
  let identitiesStoreMock;
  let documentsStoreMock;
  let dataContractsStoreMock;
  let publicKeyToIdentityIdStoreMock;
  let connectToDocumentMongoDBMock;
  let transaction;

  beforeEach(function beforeEach() {
    transaction = {
      start: this.sinon.stub(),
      commit: this.sinon.stub(),
      abort: this.sinon.stub(),
      toObject: this.sinon.stub().returns('object'),
      populateFromObject: this.sinon.stub(),
    };

    identitiesStoreMock = {
      createTransaction: this.sinon.stub().returns(transaction),
    };
    commonStoreMock = {
      createTransaction: this.sinon.stub().returns(transaction),
    };
    documentsStoreMock = {
      createTransaction: this.sinon.stub().returns(transaction),
    };
    dataContractsStoreMock = {
      createTransaction: this.sinon.stub().returns(transaction),
    };
    publicKeyToIdentityIdStoreMock = {
      createTransaction: this.sinon.stub().returns(transaction),
    };
    connectToDocumentMongoDBMock = {
      createTransaction: this.sinon.stub().returns(transaction),
    };

    blockExecutionStoreTransactions = new BlockExecutionStoreTransactions(
      commonStoreMock,
      identitiesStoreMock,
      documentsStoreMock,
      dataContractsStoreMock,
      publicKeyToIdentityIdStoreMock,
      connectToDocumentMongoDBMock,
    );

    blockExecutionStoreTransactions.transactions.documents = transaction;
  });

  describe('#start', () => {
    it('should throw BlockExecutionStoreTransactionIsAlreadyStartedError if transaction as already started', async () => {
      blockExecutionStoreTransactions.isTransactionsStarted = true;

      try {
        await blockExecutionStoreTransactions.start();

        expect.fail('should throw BlockExecutionStoreTransactionIsAlreadyStartedError');
      } catch (e) {
        expect(e).to.be.an.instanceOf(BlockExecutionStoreTransactionIsAlreadyStartedError);

        expect(transaction.start).to.be.not.called();
      }
    });

    it('should start transactions', async () => {
      await blockExecutionStoreTransactions.start();

      expect(blockExecutionStoreTransactions.isTransactionsStarted).to.be.true();

      expect(transaction.start.callCount).equals(6);
    });
  });

  describe('#commit', () => {
    it('should throw BlockExecutionStoreTransactionIsNotStartedError if transaction as already started', async () => {
      blockExecutionStoreTransactions.isTransactionsStarted = false;

      try {
        await blockExecutionStoreTransactions.commit();

        expect.fail('should throw BlockExecutionStoreTransactionIsNotStartedError');
      } catch (e) {
        expect(e).to.be.an.instanceOf(BlockExecutionStoreTransactionIsNotStartedError);

        expect(transaction.commit).to.be.not.called();
      }
    });

    it('should commit transactions', async () => {
      blockExecutionStoreTransactions.isTransactionsStarted = true;

      await blockExecutionStoreTransactions.commit();

      expect(blockExecutionStoreTransactions.isTransactionsStarted).to.be.false();

      expect(transaction.commit.callCount).equals(6);
    });
  });

  describe('#abort', () => {
    it('should throw BlockExecutionStoreTransactionIsNotStartedError if transaction as already started', async () => {
      blockExecutionStoreTransactions.isTransactionsStarted = false;

      try {
        await blockExecutionStoreTransactions.abort();

        expect.fail('should throw BlockExecutionStoreTransactionIsNotStartedError');
      } catch (e) {
        expect(e).to.be.an.instanceOf(BlockExecutionStoreTransactionIsNotStartedError);

        expect(transaction.commit).to.be.not.called();
      }
    });

    it('should commit transactions', async () => {
      blockExecutionStoreTransactions.isTransactionsStarted = true;

      await blockExecutionStoreTransactions.abort();

      expect(blockExecutionStoreTransactions.isTransactionsStarted).to.be.false();

      expect(transaction.abort.callCount).equals(6);
    });
  });

  describe('#isStarted', () => {
    it('should return is transaction is started', () => {
      expect(blockExecutionStoreTransactions.isStarted()).to.be.false();

      blockExecutionStoreTransactions.isTransactionsStarted = true;

      expect(blockExecutionStoreTransactions.isStarted()).to.be.true();
    });
  });

  describe('#getTransaction', () => {
    it('should throw BlockExecutionStoreTransactionIsNotDefinedError if transaction is not defined', async () => {
      try {
        blockExecutionStoreTransactions.getTransaction('unknown');

        expect.fail('should throw BlockExecutionStoreTransactionIsNotDefinedError');
      } catch (e) {
        expect(e).to.be.an.instanceOf(BlockExecutionStoreTransactionIsNotDefinedError);

        expect(e.getName()).to.deep.equal('unknown');
      }
    });

    it('should return transaction by name', () => {
      blockExecutionStoreTransactions.transactions.identities = 'identities';

      const result = blockExecutionStoreTransactions.getTransaction('identities');

      expect(result).to.equal('identities');
    });
  });

  describe('#toObject', () => {
    it('should return transactions as plain object', () => {
      const result = blockExecutionStoreTransactions.toObject();

      expect(result).to.deep.equal({
        common: 'object',
        identities: 'object',
        documents: 'object',
        dataContracts: 'object',
        publicKeyToIdentityId: 'object',
        assetLockTransactions: 'object',
      });
    });
  });

  describe('#populateFromObject', () => {
    it('should populate transactions from transactions object', async () => {
      const transactionsObject = {
        dataContracts: 'value',
      };

      await blockExecutionStoreTransactions.populateFromObject(transactionsObject);

      expect(transaction.populateFromObject).to.be.calledOnceWithExactly('value');
    });
  });
});
