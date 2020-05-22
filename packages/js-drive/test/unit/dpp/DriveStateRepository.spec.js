const getDocumentsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentsFixture');
const getIdentityFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityFixture');
const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');

const DriveStateRepository = require('../../../lib/dpp/DriveStateRepository');

describe('DriveStateRepository', () => {
  let stateRepository;
  let identityRepositoryMock;
  let publicKeyIdentityIdRepositoryMock;
  let dataContractRepositoryMock;
  let fetchDocumentsMock;
  let createDocumentRepositoryMock;
  let coreRpcClientMock;
  let blockExecutionDBTransactionsMock;
  let id;
  let identity;
  let documents;
  let dataContract;
  let transactionMock;

  beforeEach(function beforeEach() {
    identity = getIdentityFixture();
    documents = getDocumentsFixture();
    dataContract = getDataContractFixture();
    id = 'id';

    coreRpcClientMock = {
      getRawTransaction: this.sinon.stub(),
    };

    dataContractRepositoryMock = {
      fetch: this.sinon.stub(),
      store: this.sinon.stub(),
    };

    identityRepositoryMock = {
      fetch: this.sinon.stub(),
      store: this.sinon.stub(),
    };

    publicKeyIdentityIdRepositoryMock = {
      fetch: this.sinon.stub(),
      store: this.sinon.stub(),
    };

    blockExecutionDBTransactionsMock = {
      getTransaction: this.sinon.stub(),
    };

    fetchDocumentsMock = this.sinon.stub();

    createDocumentRepositoryMock = this.sinon.stub();

    stateRepository = new DriveStateRepository(
      identityRepositoryMock,
      publicKeyIdentityIdRepositoryMock,
      dataContractRepositoryMock,
      fetchDocumentsMock,
      createDocumentRepositoryMock,
      coreRpcClientMock,
      blockExecutionDBTransactionsMock,
    );

    transactionMock = {};

    blockExecutionDBTransactionsMock.getTransaction.returns(transactionMock);
  });

  describe('#fetchDataContract', () => {
    it('should fetch data contract from repository', async () => {
      dataContractRepositoryMock.fetch.resolves(dataContract);

      const result = await stateRepository.fetchDataContract(id);

      expect(result).to.equal(dataContract);
      expect(dataContractRepositoryMock.fetch).to.be.calledOnceWith(id);
    });
  });

  describe('#storeDataContract', () => {
    it('should store data contract to repository', async () => {
      await stateRepository.storeDataContract(dataContract);

      expect(blockExecutionDBTransactionsMock.getTransaction).to.be.calledOnceWith('dataContract');
      expect(dataContractRepositoryMock.store).to.be.calledOnceWith(dataContract, transactionMock);
    });
  });

  describe('#fetchIdentity', () => {
    it('should fetch identity from repository', async () => {
      identityRepositoryMock.fetch.resolves(identity);

      const result = await stateRepository.fetchIdentity(id);

      expect(result).to.equal(identity);
      expect(identityRepositoryMock.fetch).to.be.calledOnceWith(id, transactionMock);
      expect(blockExecutionDBTransactionsMock.getTransaction).to.be.calledOnceWith('identity');
    });
  });

  describe('#storeIdentity', () => {
    it('should store identity to repository', async () => {
      await stateRepository.storeIdentity(identity);

      expect(blockExecutionDBTransactionsMock.getTransaction).to.be.calledOnceWith('identity');
      expect(identityRepositoryMock.store).to.be.calledOnceWith(identity, transactionMock);
    });
  });

  describe('#storePublicKeyIdentityId', () => {
    it('should store public key hash and identity id pair to repository', async () => {
      await stateRepository.storePublicKeyIdentityId(
        identity.getPublicKeyById(0).hash(), identity.getId(),
      );

      expect(blockExecutionDBTransactionsMock.getTransaction).to.be.calledOnceWith('identity');
      expect(publicKeyIdentityIdRepositoryMock.store).to.have.been.calledOnceWithExactly(
        identity.getPublicKeyById(0).hash(),
        identity.getId(),
        transactionMock,
      );
    });
  });

  describe('#fetchPublicKeyIdentityId', () => {
    it('should fetch previously stored public key hash and identity id pair', async () => {
      const publicKeyHash = identity.getPublicKeyById(0).hash();

      publicKeyIdentityIdRepositoryMock
        .fetch
        .withArgs(publicKeyHash)
        .resolves(identity.getId());

      const result = await stateRepository.fetchPublicKeyIdentityId(
        identity.getPublicKeyById(0).hash(),
      );

      expect(result).to.deep.equal(identity.getId());
    });

    it('should return null if pair was not found', async () => {
      const publicKeyHash = identity.getPublicKeyById(0).hash();

      publicKeyIdentityIdRepositoryMock
        .fetch
        .withArgs(publicKeyHash)
        .resolves(null);

      const result = await stateRepository.fetchPublicKeyIdentityId(
        identity.getPublicKeyById(0).hash(),
      );

      expect(result).to.be.null();
    });
  });

  describe('#fetchDocuments', () => {
    it('should fetch documents from repository', async () => {
      const contractId = 'id';
      const type = 'documentType';
      const options = {};

      fetchDocumentsMock.resolves(documents);

      const result = await stateRepository.fetchDocuments(contractId, type, options);

      expect(result).to.equal(documents);
      expect(fetchDocumentsMock).to.be.calledOnceWith(contractId, type, options, transactionMock);
      expect(blockExecutionDBTransactionsMock.getTransaction).to.be.calledOnceWith('document');
    });
  });

  describe('#storeDocument', () => {
    it('should store document in repository', async function it() {
      const storeMock = this.sinon.stub();
      createDocumentRepositoryMock.returns({
        store: storeMock,
      });

      const [document] = documents;
      await stateRepository.storeDocument(document);

      expect(blockExecutionDBTransactionsMock.getTransaction).to.be.calledOnceWith('document');
      expect(createDocumentRepositoryMock).to.be.calledOnceWith(
        document.getDataContractId(),
        document.getType(),
      );
      expect(storeMock).to.be.calledOnceWith(document, transactionMock);
    });
  });

  describe('#removeDocument', () => {
    it('should delete document from repository', async function it() {
      const contractId = 'contractId';
      const type = 'documentType';

      const deleteMock = this.sinon.stub();
      createDocumentRepositoryMock.returns({
        delete: deleteMock,
      });

      await stateRepository.removeDocument(contractId, type, id);

      expect(blockExecutionDBTransactionsMock.getTransaction).to.be.calledOnceWith('document');
      expect(createDocumentRepositoryMock).to.be.calledOnceWith(contractId, type);
      expect(deleteMock).to.be.calledOnceWith(id, transactionMock);
    });
  });

  describe('#fetchTransaction', () => {
    it('should fetch transaction from core', async () => {
      const rawTransaction = {
        data: 'some result',
      };

      coreRpcClientMock.getRawTransaction.resolves({ result: rawTransaction });

      const result = await stateRepository.fetchTransaction(id);

      expect(result).to.deep.equal(rawTransaction);
      expect(coreRpcClientMock.getRawTransaction).to.be.calledOnceWithExactly(id, 1);
    });

    it('should return null if core throws Invalid address or key error', async () => {
      const error = new Error('Some error');
      error.code = -5;

      coreRpcClientMock.getRawTransaction.throws(error);

      const result = await stateRepository.fetchTransaction(id);

      expect(result).to.equal(null);
      expect(coreRpcClientMock.getRawTransaction).to.be.calledOnceWith(id);
    });

    it('should throw an error if core throws an unknown error', async () => {
      const error = new Error('Some error');

      coreRpcClientMock.getRawTransaction.throws(error);

      try {
        await stateRepository.fetchTransaction(id);

        expect.fail('should throw error');
      } catch (e) {
        expect(e).to.equal(error);
        expect(coreRpcClientMock.getRawTransaction).to.be.calledOnceWith(id);
      }
    });
  });
});
