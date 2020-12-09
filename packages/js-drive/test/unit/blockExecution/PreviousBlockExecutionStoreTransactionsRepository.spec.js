const cbor = require('cbor');

const PreviousBlockExecutionStoreTransactionsRepository = require('../../../lib/blockExecution/PreviousBlockExecutionStoreTransactionsRepository');
const BlockExecutionStoreTransactionIsNotStartedError = require('../../../lib/blockExecution/errors/BlockExecutionStoreTransactionIsNotStartedError');

describe('PreviousBlockExecutionStoreTransactionsRepository', () => {
  let previousBlockExecutionStoreTransactionsRepository;
  let previousBlockExecutionTransactionDBMock;
  let transactions;
  let serializedTransactions;
  let identity;
  let previousCommonStore;
  let previousIdentitiesStore;
  let previousDocumentsStore;
  let previousDataContractsStore;
  let previousPublicKeyToIdentityIdStore;
  let previousConnectToDocumentMongoDB;

  beforeEach(function beforeEach() {
    previousBlockExecutionTransactionDBMock = {
      set: this.sinon.stub(),
      get: this.sinon.stub(),
      clear: this.sinon.stub(),
    };

    previousCommonStore = 'previousCommonStore';
    previousIdentitiesStore = 'previousIdentitiesStore';
    previousDocumentsStore = 'previousDocumentsStore';
    previousDataContractsStore = 'previousDataContractsStore';
    previousPublicKeyToIdentityIdStore = 'previousPublicKeyToIdentityIdStore';
    previousConnectToDocumentMongoDB = 'previousConnectToDocumentMongoDB';

    previousBlockExecutionStoreTransactionsRepository = new
    PreviousBlockExecutionStoreTransactionsRepository(
      previousBlockExecutionTransactionDBMock,
      previousCommonStore,
      previousIdentitiesStore,
      previousDocumentsStore,
      previousDataContractsStore,
      previousPublicKeyToIdentityIdStore,
      previousConnectToDocumentMongoDB,
    );

    identity = 'identity';

    transactions = {
      identity,
    };

    serializedTransactions = cbor.encode(transactions);
  });

  describe('#store', () => {
    let storeTransactionsMock;

    beforeEach(function beforeEach() {
      storeTransactionsMock = {
        isStarted: this.sinon.stub(),
        toObject: this.sinon.stub().returns(transactions),
      };
    });

    it('should throw BlockExecutionStoreTransactionIsNotStartedError is transaction is not started', async () => {
      storeTransactionsMock.isStarted.returns(false);

      try {
        await previousBlockExecutionStoreTransactionsRepository.store(storeTransactionsMock);

        expect.fail('should throw BlockExecutionStoreTransactionIsNotStartedError');
      } catch (e) {
        expect(e).to.be.an.instanceOf(BlockExecutionStoreTransactionIsNotStartedError);
      }
    });

    it('should store transactions into DB', async () => {
      storeTransactionsMock.isStarted.returns(true);

      await previousBlockExecutionStoreTransactionsRepository.store(storeTransactionsMock);

      expect(previousBlockExecutionTransactionDBMock.set).to.be.calledOnceWithExactly(
        serializedTransactions,
      );
    });
  });

  describe('#fetch', () => {
    it('should do nothing if there are no stored transactions', async () => {
      previousBlockExecutionTransactionDBMock.get.returns(null);

      const result = await previousBlockExecutionStoreTransactionsRepository.fetch(transactions);

      expect(result).to.be.null();
    });
  });

  describe('#clear', () => {
    it('should clear DB state', async () => {
      previousBlockExecutionStoreTransactionsRepository.clear();

      expect(previousBlockExecutionTransactionDBMock.clear).to.be.calledOnce();
    });
  });
});
