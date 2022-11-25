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

  describe('#updateIdentity', () => {
    let identity;

    beforeEach(() => {
      identity = getIdentityFixture();
    });

    it('should call logger with proper params', async () => {
      const response = undefined;

      stateRepositoryMock.updateIdentity.resolves(response);

      await loggedStateRepositoryDecorator.updateIdentity(identity);

      expect(loggerMock.trace).to.be.calledOnceWithExactly({
        stateRepository: {
          method: 'updateIdentity',
          parameters: { identity },
          response,
        },
      }, 'StateRepository#updateIdentity');
    });

    it('should call logger in case of error', async () => {
      const error = new Error('unknown error');

      stateRepositoryMock.updateIdentity.throws(error);

      try {
        await loggedStateRepositoryDecorator.updateIdentity(identity);

        expect.fail('should throw an error');
      } catch (e) {
        expect(e).equals(error);
      }

      expect(loggerMock.trace).to.be.calledOnceWithExactly({
        stateRepository: {
          method: 'updateIdentity',
          parameters: { identity },
          response: undefined,
        },
      }, 'StateRepository#updateIdentity');
    });
  });

  describe('#storeIdentityPublicKeyHashes', () => {
    let identityId;
    let publicKeyHashes;

    beforeEach(() => {
      identityId = generateRandomIdentifier();
      publicKeyHashes = [Buffer.alloc(36), Buffer.alloc(36)];
    });

    it('should call logger with proper params', async () => {
      const response = undefined;

      stateRepositoryMock.storeIdentityPublicKeyHashes.resolves(response);

      await loggedStateRepositoryDecorator
        .storeIdentityPublicKeyHashes(identityId, publicKeyHashes);

      expect(loggerMock.trace).to.be.calledOnceWithExactly({
        stateRepository: {
          method: 'storeIdentityPublicKeyHashes',
          parameters: {
            identityId,
            publicKeyHashes: publicKeyHashes.map((hash) => hash.toString('base64')),
          },
          response,
        },
      }, 'StateRepository#storeIdentityPublicKeyHashes');
    });

    it('should call logger in case of error', async () => {
      const error = new Error('unknown error');

      stateRepositoryMock.storeIdentityPublicKeyHashes.throws(error);

      try {
        await loggedStateRepositoryDecorator
          .storeIdentityPublicKeyHashes(identityId, publicKeyHashes);

        expect.fail('should throw an error');
      } catch (e) {
        expect(e).equals(error);
      }

      expect(loggerMock.trace).to.be.calledOnceWithExactly({
        stateRepository: {
          method: 'storeIdentityPublicKeyHashes',
          parameters: {
            identityId,
            publicKeyHashes: publicKeyHashes.map((hash) => hash.toString('base64')),
          },
          response: undefined,
        },
      }, 'StateRepository#storeIdentityPublicKeyHashes');
    });
  });

  describe('#fetchIdentityIdsByPublicKeyHashes', () => {
    let publicKeyHashes;

    beforeEach(() => {
      publicKeyHashes = [Buffer.alloc(36), Buffer.alloc(36)];
    });

    it('should call logger with proper params', async () => {
      const response = [null, generateRandomIdentifier()];

      stateRepositoryMock.fetchIdentityIdsByPublicKeyHashes.resolves(response);

      await loggedStateRepositoryDecorator.fetchIdentityIdsByPublicKeyHashes(publicKeyHashes);

      expect(loggerMock.trace).to.be.calledOnceWithExactly({
        stateRepository: {
          method: 'fetchIdentityIdsByPublicKeyHashes',
          parameters: {
            publicKeyHashes: publicKeyHashes.map((hash) => hash.toString('base64')),
          },
          response,
        },
      }, 'StateRepository#fetchIdentityIdsByPublicKeyHashes');
    });

    it('should call logger in case of error', async () => {
      const error = new Error('unknown error');

      stateRepositoryMock.fetchIdentityIdsByPublicKeyHashes.throws(error);

      try {
        await loggedStateRepositoryDecorator.fetchIdentityIdsByPublicKeyHashes(publicKeyHashes);

        expect.fail('should throw an error');
      } catch (e) {
        expect(e).equals(error);
      }

      expect(loggerMock.trace).to.be.calledOnceWithExactly({
        stateRepository: {
          method: 'fetchIdentityIdsByPublicKeyHashes',
          parameters: {
            publicKeyHashes: publicKeyHashes.map((hash) => hash.toString('base64')),
          },
          response: undefined,
        },
      }, 'StateRepository#fetchIdentityIdsByPublicKeyHashes');
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

  describe('#fetchLatestPlatformBlockHeader', () => {
    it('should call logger with proper params', async () => {
      const response = { };

      stateRepositoryMock.fetchLatestPlatformBlockHeader.resolves(response);

      await loggedStateRepositoryDecorator.fetchLatestPlatformBlockHeader();

      expect(loggerMock.trace).to.be.calledOnceWithExactly({
        stateRepository: {
          method: 'fetchLatestPlatformBlockHeader',
          parameters: { },
          response,
        },
      }, 'StateRepository#fetchLatestPlatformBlockHeader');
    });

    it('should call logger in case of error', async () => {
      const error = new Error('unknown error');

      stateRepositoryMock.fetchLatestPlatformBlockHeader.throws(error);

      try {
        await loggedStateRepositoryDecorator.fetchLatestPlatformBlockHeader();

        expect.fail('should throw an error');
      } catch (e) {
        expect(e).equals(error);
      }

      expect(loggerMock.trace).to.be.calledOnceWithExactly({
        stateRepository: {
          method: 'fetchLatestPlatformBlockHeader',
          parameters: { },
          response: undefined,
        },
      }, 'StateRepository#fetchLatestPlatformBlockHeader');
    });
  });
});
