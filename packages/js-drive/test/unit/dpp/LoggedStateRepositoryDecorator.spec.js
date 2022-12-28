const getIdentityFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityFixture');
const createStateRepositoryMock = require('@dashevo/dpp/lib/test/mocks/createStateRepositoryMock');
const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const getDocumentsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentsFixture');
const generateRandomIdentifier = require('@dashevo/dpp/lib/test/utils/generateRandomIdentifier');

const LoggedStateRepositoryDecorator = require('../../../lib/dpp/LoggedStateRepositoryDecorator');
const LoggerMock = require('../../../lib/test/mock/LoggerMock');
const BlockExecutionContextMock = require('../../../lib/test/mock/BlockExecutionContextMock');

describe('LoggedStateRepositoryDecorator', () => {
  let loggedStateRepositoryDecorator;
  let stateRepositoryMock;
  let loggerMock;
  let blockExecutionContextMock;

  beforeEach(function beforeEach() {
    stateRepositoryMock = createStateRepositoryMock(this.sinon);
    loggerMock = new LoggerMock(this.sinon);

    blockExecutionContextMock = new BlockExecutionContextMock(this.sinon);
    blockExecutionContextMock.getConsensusLogger.returns(loggerMock);

    loggedStateRepositoryDecorator = new LoggedStateRepositoryDecorator(
      stateRepositoryMock,
      blockExecutionContextMock,
    );
  });

  describe('#fetchIdentity', () => {
    let id;

    beforeEach(() => {
      id = generateRandomIdentifier();
    });

    it('should call logger with proper params', async () => {
      const response = getIdentityFixture();

      stateRepositoryMock.fetchIdentity.resolves(response);

      await loggedStateRepositoryDecorator.fetchIdentity(id);

      expect(loggerMock.trace).to.be.calledOnceWithExactly({
        stateRepository: {
          method: 'fetchIdentity',
          parameters: { id },
          response,
        },
      }, 'StateRepository#fetchIdentity');
    });

    it('should call logger in case of error', async () => {
      const error = new Error('unknown error');

      stateRepositoryMock.fetchIdentity.throws(error);

      try {
        await loggedStateRepositoryDecorator.fetchIdentity(id);

        expect.fail('should throw an error');
      } catch (e) {
        expect(e).equals(error);
      }

      expect(loggerMock.trace).to.be.calledOnceWithExactly({
        stateRepository: {
          method: 'fetchIdentity',
          parameters: { id },
          response: undefined,
        },
      }, 'StateRepository#fetchIdentity');
    });
  });

  describe('#storeIdentity', () => {
    let identity;

    beforeEach(() => {
      identity = getIdentityFixture();
    });

    it('should call logger with proper params', async () => {
      const response = undefined;

      stateRepositoryMock.createIdentity.resolves(response);

      await loggedStateRepositoryDecorator.createIdentity(identity);

      expect(loggerMock.trace).to.be.calledOnceWithExactly({
        stateRepository: {
          method: 'createIdentity',
          parameters: { identity },
          response,
        },
      }, 'StateRepository#createIdentity');
    });

    it('should call logger in case of error', async () => {
      const error = new Error('unknown error');

      stateRepositoryMock.createIdentity.throws(error);

      try {
        await loggedStateRepositoryDecorator.createIdentity(identity);

        expect.fail('should throw an error');
      } catch (e) {
        expect(e).equals(error);
      }

      expect(loggerMock.trace).to.be.calledOnceWithExactly({
        stateRepository: {
          method: 'createIdentity',
          parameters: { identity },
          response: undefined,
        },
      }, 'StateRepository#createIdentity');
    });
  });

  describe('#addKeysToIdentity', () => {
    let identity;

    beforeEach(() => {
      identity = getIdentityFixture();
    });

    it('should call logger with proper params', async () => {
      const response = undefined;

      stateRepositoryMock.addKeysToIdentity.resolves(response);

      await loggedStateRepositoryDecorator.addKeysToIdentity(
        identity.getId(),
        identity.getPublicKeys(),
      );

      expect(loggerMock.trace).to.be.calledOnceWithExactly({
        stateRepository: {
          method: 'addKeysToIdentity',
          parameters: { identityId: identity.getId(), keys: identity.getPublicKeys() },
          response,
        },
      }, 'StateRepository#addKeysToIdentity');
    });

    it('should call logger in case of error', async () => {
      const error = new Error('unknown error');

      stateRepositoryMock.addKeysToIdentity.throws(error);

      try {
        await loggedStateRepositoryDecorator.addKeysToIdentity(identity.getId(), identity.getPublicKeys());

        expect.fail('should throw an error');
      } catch (e) {
        expect(e).equals(error);
      }

      expect(loggerMock.trace).to.be.calledOnceWithExactly({
        stateRepository: {
          method: 'addKeysToIdentity',
          parameters: { identityId: identity.getId(), keys: identity.getPublicKeys() },
          response: undefined,
        },
      }, 'StateRepository#addKeysToIdentity');
    });
  });

  describe('#addToIdentityBalance', () => {
    let identity;

    beforeEach(() => {
      identity = getIdentityFixture();
    });

    it('should call logger with proper params', async () => {
      const response = undefined;

      stateRepositoryMock.addToIdentityBalance.resolves(response);

      const amount = 200;

      await loggedStateRepositoryDecorator.addToIdentityBalance(
        identity.getId(),
        amount,
      );

      expect(loggerMock.trace).to.be.calledOnceWithExactly({
        stateRepository: {
          method: 'addToIdentityBalance',
          parameters: { identityId: identity.getId(), amount },
          response,
        },
      }, 'StateRepository#addToIdentityBalance');
    });

    it('should call logger in case of error', async () => {
      const error = new Error('unknown error');

      stateRepositoryMock.addToIdentityBalance.throws(error);

      const amount = 100;

      try {
        await loggedStateRepositoryDecorator.addToIdentityBalance(identity.getId(), amount);

        expect.fail('should throw an error');
      } catch (e) {
        expect(e).equals(error);
      }

      expect(loggerMock.trace).to.be.calledOnceWithExactly({
        stateRepository: {
          method: 'addToIdentityBalance',
          parameters: { identityId: identity.getId(), amount },
          response: undefined,
        },
      }, 'StateRepository#addToIdentityBalance');
    });
  });

  describe('#disableIdentityKeys', () => {
    let identity;

    beforeEach(() => {
      identity = getIdentityFixture();
    });

    it('should call logger with proper params', async () => {
      const response = undefined;

      stateRepositoryMock.disableIdentityKeys.resolves(response);

      const keyIds = [1, 2];
      const disableAt = 123;

      await loggedStateRepositoryDecorator.disableIdentityKeys(
        identity.getId(),
        keyIds,
        disableAt,
      );

      expect(loggerMock.trace).to.be.calledOnceWithExactly({
        stateRepository: {
          method: 'disableIdentityKeys',
          parameters: { identityId: identity.getId(), keyIds, disableAt },
          response,
        },
      }, 'StateRepository#disableIdentityKeys');
    });

    it('should call logger in case of error', async () => {
      const error = new Error('unknown error');

      stateRepositoryMock.disableIdentityKeys.throws(error);

      const keyIds = [1, 2];
      const disableAt = 123;

      try {
        await loggedStateRepositoryDecorator.disableIdentityKeys(
          identity.getId(),
          keyIds,
          disableAt,
        );

        expect.fail('should throw an error');
      } catch (e) {
        expect(e).equals(error);
      }

      expect(loggerMock.trace).to.be.calledOnceWithExactly({
        stateRepository: {
          method: 'disableIdentityKeys',
          parameters: { identityId: identity.getId(), keyIds, disableAt },
          response: undefined,
        },
      }, 'StateRepository#disableIdentityKeys');
    });
  });

  describe('#updateIdentityRevision', () => {
    let identity;

    beforeEach(() => {
      identity = getIdentityFixture();
    });

    it('should call logger with proper params', async () => {
      const response = undefined;

      stateRepositoryMock.updateIdentityRevision.resolves(response);

      const revision = 1;

      await loggedStateRepositoryDecorator.updateIdentityRevision(
        identity.getId(),
        revision,
      );

      expect(loggerMock.trace).to.be.calledOnceWithExactly({
        stateRepository: {
          method: 'updateIdentityRevision',
          parameters: { identityId: identity.getId(), revision },
          response,
        },
      }, 'StateRepository#updateIdentityRevision');
    });

    it('should call logger in case of error', async () => {
      const error = new Error('unknown error');

      stateRepositoryMock.updateIdentityRevision.throws(error);

      const revision = 1;

      try {
        await loggedStateRepositoryDecorator.updateIdentityRevision(
          identity.getId(),
          revision,
        );

        expect.fail('should throw an error');
      } catch (e) {
        expect(e).equals(error);
      }

      expect(loggerMock.trace).to.be.calledOnceWithExactly({
        stateRepository: {
          method: 'updateIdentityRevision',
          parameters: { identityId: identity.getId(), revision },
          response: undefined,
        },
      }, 'StateRepository#updateIdentityRevision');
    });
  });

  describe('#fetchDataContract', () => {
    let id;

    beforeEach(() => {
      id = generateRandomIdentifier();
    });

    it('should call logger with proper params', async () => {
      const response = getDataContractFixture();

      stateRepositoryMock.fetchDataContract.resolves(response);

      await loggedStateRepositoryDecorator.fetchDataContract(id);

      expect(loggerMock.trace).to.be.calledOnceWithExactly({
        stateRepository: {
          method: 'fetchDataContract',
          parameters: { id },
          response,
        },
      }, 'StateRepository#fetchDataContract');
    });

    it('should call logger in case of error', async () => {
      const error = new Error('unknown error');

      stateRepositoryMock.fetchDataContract.throws(error);

      try {
        await loggedStateRepositoryDecorator.fetchDataContract(id);

        expect.fail('should throw an error');
      } catch (e) {
        expect(e).equals(error);
      }

      expect(loggerMock.trace).to.be.calledOnceWithExactly({
        stateRepository: {
          method: 'fetchDataContract',
          parameters: { id },
          response: undefined,
        },
      }, 'StateRepository#fetchDataContract');
    });
  });

  describe('#createDataContract', () => {
    let dataContract;

    beforeEach(() => {
      dataContract = getDataContractFixture();
    });

    it('should call logger with proper params', async () => {
      const response = undefined;

      stateRepositoryMock.createDataContract.resolves(response);

      await loggedStateRepositoryDecorator.createDataContract(dataContract);

      expect(loggerMock.trace).to.be.calledOnceWithExactly({
        stateRepository: {
          method: 'createDataContract',
          parameters: { dataContract },
          response,
        },
      }, 'StateRepository#createDataContract');
    });

    it('should call logger in case of error', async () => {
      const error = new Error('unknown error');

      stateRepositoryMock.createDataContract.throws(error);

      try {
        await loggedStateRepositoryDecorator.createDataContract(dataContract);

        expect.fail('should throw an error');
      } catch (e) {
        expect(e).equals(error);
      }

      expect(loggerMock.trace).to.be.calledOnceWithExactly({
        stateRepository: {
          method: 'createDataContract',
          parameters: { dataContract },
          response: undefined,
        },
      }, 'StateRepository#createDataContract');
    });
  });

  describe('#updateDataContract', () => {
    let dataContract;

    beforeEach(() => {
      dataContract = getDataContractFixture();
    });

    it('should call logger with proper params', async () => {
      const response = undefined;

      stateRepositoryMock.updateDataContract.resolves(response);

      await loggedStateRepositoryDecorator.updateDataContract(dataContract);

      expect(loggerMock.trace).to.be.calledOnceWithExactly({
        stateRepository: {
          method: 'updateDataContract',
          parameters: { dataContract },
          response,
        },
      }, 'StateRepository#updateDataContract');
    });

    it('should call logger in case of error', async () => {
      const error = new Error('unknown error');

      stateRepositoryMock.updateDataContract.throws(error);

      try {
        await loggedStateRepositoryDecorator.updateDataContract(dataContract);

        expect.fail('should throw an error');
      } catch (e) {
        expect(e).equals(error);
      }

      expect(loggerMock.trace).to.be.calledOnceWithExactly({
        stateRepository: {
          method: 'updateDataContract',
          parameters: { dataContract },
          response: undefined,
        },
      }, 'StateRepository#updateDataContract');
    });
  });

  describe('#fetchDocuments', () => {
    let contractId;
    let type;
    let options;

    beforeEach(() => {
      contractId = generateRandomIdentifier();
      type = 'type';
      options = {
        where: [['field', '==', 'value']],
      };
    });

    it('should call logger with proper params', async () => {
      const response = getDocumentsFixture();

      stateRepositoryMock.fetchDocuments.resolves(response);

      await loggedStateRepositoryDecorator.fetchDocuments(contractId, type, options);

      expect(loggerMock.trace).to.be.calledOnceWithExactly({
        stateRepository: {
          method: 'fetchDocuments',
          parameters: { contractId, type, options },
          response,
        },
      }, 'StateRepository#fetchDocuments');
    });

    it('should call logger in case of error', async () => {
      const error = new Error('unknown error');

      stateRepositoryMock.fetchDocuments.throws(error);

      try {
        await loggedStateRepositoryDecorator.fetchDocuments(contractId, type, options);

        expect.fail('should throw an error');
      } catch (e) {
        expect(e).equals(error);
      }

      expect(loggerMock.trace).to.be.calledOnceWithExactly({
        stateRepository: {
          method: 'fetchDocuments',
          parameters: { contractId, type, options },
          response: undefined,
        },
      }, 'StateRepository#fetchDocuments');
    });
  });

  describe('#createDocument', () => {
    let document;

    beforeEach(() => {
      [document] = getDocumentsFixture();
    });

    it('should call logger with proper params', async () => {
      const response = undefined;

      stateRepositoryMock.createDocument.resolves(response);

      await loggedStateRepositoryDecorator.createDocument(document);

      expect(loggerMock.trace).to.be.calledOnceWithExactly({
        stateRepository: {
          method: 'createDocument',
          parameters: { document },
          response,
        },
      }, 'StateRepository#createDocument');
    });

    it('should call logger in case of error', async () => {
      const error = new Error('unknown error');

      stateRepositoryMock.createDocument.throws(error);

      try {
        await loggedStateRepositoryDecorator.createDocument(document);

        expect.fail('should throw an error');
      } catch (e) {
        expect(e).equals(error);
      }

      expect(loggerMock.trace).to.be.calledOnceWithExactly({
        stateRepository: {
          method: 'createDocument',
          parameters: { document },
          response: undefined,
        },
      }, 'StateRepository#createDocument');
    });
  });

  describe('#updateDocument', () => {
    let document;

    beforeEach(() => {
      [document] = getDocumentsFixture();
    });

    it('should call logger with proper params', async () => {
      const response = undefined;

      stateRepositoryMock.updateDocument.resolves(response);

      await loggedStateRepositoryDecorator.updateDocument(document);

      expect(loggerMock.trace).to.be.calledOnceWithExactly({
        stateRepository: {
          method: 'updateDocument',
          parameters: { document },
          response,
        },
      }, 'StateRepository#updateDocument');
    });

    it('should call logger in case of error', async () => {
      const error = new Error('unknown error');

      stateRepositoryMock.updateDocument.throws(error);

      try {
        await loggedStateRepositoryDecorator.updateDocument(document);

        expect.fail('should throw an error');
      } catch (e) {
        expect(e).equals(error);
      }

      expect(loggerMock.trace).to.be.calledOnceWithExactly({
        stateRepository: {
          method: 'updateDocument',
          parameters: { document },
          response: undefined,
        },
      }, 'StateRepository#updateDocument');
    });
  });

  describe('#removeDocument', () => {
    let dataContract;
    let type;
    let id;

    beforeEach(() => {
      dataContract = getDataContractFixture();
      type = 'type';
      id = generateRandomIdentifier();
    });

    it('should call logger with proper params', async () => {
      const response = undefined;

      stateRepositoryMock.removeDocument.resolves(response);

      await loggedStateRepositoryDecorator.removeDocument(dataContract, type, id);

      expect(loggerMock.trace).to.be.calledOnceWithExactly({
        stateRepository: {
          method: 'removeDocument',
          parameters: { dataContract, type, id },
          response,
        },
      }, 'StateRepository#removeDocument');
    });

    it('should call logger in case of error', async () => {
      const error = new Error('unknown error');

      stateRepositoryMock.removeDocument.throws(error);

      try {
        await loggedStateRepositoryDecorator.removeDocument(dataContract, type, id);

        expect.fail('should throw an error');
      } catch (e) {
        expect(e).equals(error);
      }

      expect(loggerMock.trace).to.be.calledOnceWithExactly({
        stateRepository: {
          method: 'removeDocument',
          parameters: { dataContract, type, id },
          response: undefined,
        },
      }, 'StateRepository#removeDocument');
    });
  });

  describe('#fetchTransaction', () => {
    let id;

    beforeEach(() => {
      id = 'id';
    });

    it('should call logger with proper params', async () => {
      const response = { hex: '123' };

      stateRepositoryMock.fetchTransaction.resolves(response);

      await loggedStateRepositoryDecorator.fetchTransaction(id);

      expect(loggerMock.trace).to.be.calledOnceWithExactly({
        stateRepository: {
          method: 'fetchTransaction',
          parameters: { id },
          response,
        },
      }, 'StateRepository#fetchTransaction');
    });

    it('should call logger in case of error', async () => {
      const error = new Error('unknown error');

      stateRepositoryMock.fetchTransaction.throws(error);

      try {
        await loggedStateRepositoryDecorator.fetchTransaction(id);

        expect.fail('should throw an error');
      } catch (e) {
        expect(e).equals(error);
      }

      expect(loggerMock.trace).to.be.calledOnceWithExactly({
        stateRepository: {
          method: 'fetchTransaction',
          parameters: { id },
          response: undefined,
        },
      }, 'StateRepository#fetchTransaction');
    });
  });

  describe('#fetchLatestPlatformBlockHeight', () => {
    it('should call logger with proper params', async () => {
      const response = {};

      stateRepositoryMock.fetchLatestPlatformBlockHeight.resolves(response);

      await loggedStateRepositoryDecorator.fetchLatestPlatformBlockHeight();

      expect(loggerMock.trace).to.be.calledOnceWithExactly({
        stateRepository: {
          method: 'fetchLatestPlatformBlockHeight',
          parameters: {},
          response,
        },
      }, 'StateRepository#fetchLatestPlatformBlockHeight');
    });

    it('should call logger in case of error', async () => {
      const error = new Error('unknown error');

      stateRepositoryMock.fetchLatestPlatformBlockHeight.throws(error);

      try {
        await loggedStateRepositoryDecorator.fetchLatestPlatformBlockHeight();

        expect.fail('should throw an error');
      } catch (e) {
        expect(e).equals(error);
      }

      expect(loggerMock.trace).to.be.calledOnceWithExactly({
        stateRepository: {
          method: 'fetchLatestPlatformBlockHeight',
          parameters: {},
          response: undefined,
        },
      }, 'StateRepository#fetchLatestPlatformBlockHeight');
    });
  });

  describe('#fetchLatestPlatformBlockTime', () => {
    it('should call logger with proper params', async () => {
      const response = {};

      stateRepositoryMock.fetchLatestPlatformBlockTime.resolves(response);

      await loggedStateRepositoryDecorator.fetchLatestPlatformBlockTime();

      expect(loggerMock.trace).to.be.calledOnceWithExactly({
        stateRepository: {
          method: 'fetchLatestPlatformBlockTime',
          parameters: {},
          response,
        },
      }, 'StateRepository#fetchLatestPlatformBlockTime');
    });

    it('should call logger in case of error', async () => {
      const error = new Error('unknown error');

      stateRepositoryMock.fetchLatestPlatformBlockTime.throws(error);

      try {
        await loggedStateRepositoryDecorator.fetchLatestPlatformBlockTime();

        expect.fail('should throw an error');
      } catch (e) {
        expect(e).equals(error);
      }

      expect(loggerMock.trace).to.be.calledOnceWithExactly({
        stateRepository: {
          method: 'fetchLatestPlatformBlockTime',
          parameters: {},
          response: undefined,
        },
      }, 'StateRepository#fetchLatestPlatformBlockTime');
    });
  });

  describe('#fetchLatestPlatformCoreChainLockedHeight', () => {
    it('should call logger with proper params', async () => {
      const response = {};

      stateRepositoryMock.fetchLatestPlatformCoreChainLockedHeight.resolves(response);

      await loggedStateRepositoryDecorator.fetchLatestPlatformCoreChainLockedHeight();

      expect(loggerMock.trace).to.be.calledOnceWithExactly({
        stateRepository: {
          method: 'fetchLatestPlatformCoreChainLockedHeight',
          parameters: {},
          response,
        },
      }, 'StateRepository#fetchLatestPlatformCoreChainLockedHeight');
    });

    it('should call logger in case of error', async () => {
      const error = new Error('unknown error');

      stateRepositoryMock.fetchLatestPlatformCoreChainLockedHeight.throws(error);

      try {
        await loggedStateRepositoryDecorator.fetchLatestPlatformCoreChainLockedHeight();

        expect.fail('should throw an error');
      } catch (e) {
        expect(e).equals(error);
      }

      expect(loggerMock.trace).to.be.calledOnceWithExactly({
        stateRepository: {
          method: 'fetchLatestPlatformCoreChainLockedHeight',
          parameters: {},
          response: undefined,
        },
      }, 'StateRepository#fetchLatestPlatformCoreChainLockedHeight');
    });
  });

  describe('#fetchLatestWithdrawalTransactionIndex', () => {
    it('should call fetchLatestWithdrawalTransactionIndex', async () => {
      stateRepositoryMock.fetchLatestWithdrawalTransactionIndex.resolves(42);

      const result = await loggedStateRepositoryDecorator.fetchLatestWithdrawalTransactionIndex();

      expect(result).to.equal(42);
      expect(
        stateRepositoryMock.fetchLatestWithdrawalTransactionIndex,
      ).to.have.been.calledOnce();

      expect(loggerMock.trace).to.be.calledOnceWithExactly({
        stateRepository: {
          method: 'fetchLatestWithdrawalTransactionIndex',
          parameters: {},
          response: 42,
        },
      }, 'StateRepository#fetchLatestWithdrawalTransactionIndex');
    });
  });

  describe('#enqueueWithdrawalTransaction', () => {
    it('should call enqueueWithdrawalTransaction', async () => {
      const index = 42;
      const transactionBytes = Buffer.alloc(32, 1);

      await loggedStateRepositoryDecorator.enqueueWithdrawalTransaction(
        index, transactionBytes,
      );

      expect(
        stateRepositoryMock.enqueueWithdrawalTransaction,
      ).to.have.been.calledOnceWithExactly(
        index,
        transactionBytes,
      );

      expect(loggerMock.trace).to.be.calledOnceWithExactly({
        stateRepository: {
          method: 'enqueueWithdrawalTransaction',
          parameters: { index, transactionBytes },
          response: undefined,
        },
      }, 'StateRepository#enqueueWithdrawalTransaction');
    });
  });
});
