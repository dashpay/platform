const getDocumentsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentsFixture');
const getIdentityFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityFixture');
const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');

const CachedStateRepositoryDecorator = require('../../../lib/dpp/CachedStateRepositoryDecorator');

describe('CachedStateRepositoryDecorator', () => {
  let stateRepositoryMock;
  let cachedStateRepository;
  let dataContractCacheMock;
  let id;
  let identity;
  let documents;
  let dataContract;

  beforeEach(function beforeEach() {
    id = 'id';
    identity = getIdentityFixture();
    documents = getDocumentsFixture();
    dataContract = getDataContractFixture();

    dataContractCacheMock = {
      set: this.sinon.stub(),
      get: this.sinon.stub(),
    };

    stateRepositoryMock = {
      fetchIdentity: this.sinon.stub(),
      fetchDocuments: this.sinon.stub(),
      fetchTransaction: this.sinon.stub(),
      fetchDataContract: this.sinon.stub(),
      storeIdentity: this.sinon.stub(),
      storeDocument: this.sinon.stub(),
      removeDocument: this.sinon.stub(),
    };

    cachedStateRepository = new CachedStateRepositoryDecorator(
      stateRepositoryMock,
      dataContractCacheMock,
    );
  });

  describe('#fetchIdentity', () => {
    it('should fetch identity from state repository', async () => {
      stateRepositoryMock.fetchIdentity.resolves(identity);

      const result = await cachedStateRepository.fetchIdentity(id);

      expect(result).to.deep.equal(identity);
      expect(stateRepositoryMock.fetchIdentity).to.be.calledOnceWith(id);
    });
  });

  describe('#storeIdentity', () => {
    it('should store identity to repository', async () => {
      await cachedStateRepository.storeIdentity(identity);

      expect(stateRepositoryMock.storeIdentity).to.be.calledOnceWith(identity);
    });
  });

  describe('#fetchDocuments', () => {
    it('should fetch documents from state repository', async () => {
      const contractId = 'contractId';
      const type = 'documentType';
      const options = {};

      stateRepositoryMock.fetchDocuments.resolves(documents);

      const result = await cachedStateRepository.fetchDocuments(contractId, type, options);

      expect(result).to.equal(documents);
      expect(stateRepositoryMock.fetchDocuments).to.be.calledOnceWith(contractId, type, options);
    });
  });

  describe('#storeDocument', () => {
    it('should store document in repository', async () => {
      const [document] = documents;

      await cachedStateRepository.storeDocument(document);

      expect(stateRepositoryMock.storeDocument).to.be.calledOnceWith(document);
    });
  });

  describe('#removeDocument', () => {
    it('should delete document from repository', async () => {
      const contractId = 'contractId';
      const type = 'documentType';

      await cachedStateRepository.removeDocument(contractId, type, id);

      expect(stateRepositoryMock.removeDocument).to.be.calledOnceWith(contractId, type, id);
    });
  });

  describe('fetchTransaction', () => {
    it('should fetch transaction from state repository', async () => {
      stateRepositoryMock.fetchTransaction.resolves(dataContract);

      const result = await cachedStateRepository.fetchTransaction(id);

      expect(result).to.equal(dataContract);
      expect(stateRepositoryMock.fetchTransaction).to.be.calledOnceWith(id);
    });
  });

  describe('#fetchDataContract', () => {
    it('should fetch data contract from cache', async () => {
      dataContractCacheMock.get.returns(dataContract);

      const result = await cachedStateRepository.fetchDataContract(id);

      expect(result).to.equal(dataContract);
      expect(stateRepositoryMock.fetchDataContract).to.be.not.called();
      expect(dataContractCacheMock.get).to.be.calledOnceWith(id);
    });

    it('should fetch data contract from state repository if it is not present in cache', async () => {
      dataContractCacheMock.get.returns(undefined);
      stateRepositoryMock.fetchDataContract.resolves(dataContract);

      const result = await cachedStateRepository.fetchDataContract(id);

      expect(result).to.equal(dataContract);
      expect(dataContractCacheMock.get).to.be.calledOnceWith(id);
      expect(dataContractCacheMock.set).to.be.calledOnceWith(id, dataContract);
      expect(stateRepositoryMock.fetchDataContract).to.be.calledOnceWith(id);
    });

    it('should not store null in cache if data contract is not present in state repository', async () => {
      stateRepositoryMock.fetchDataContract.resolves(null);

      const result = await cachedStateRepository.fetchDataContract(id);

      expect(result).to.be.null();

      expect(dataContractCacheMock.get).to.be.calledOnceWith(id);
      expect(dataContractCacheMock.set).to.not.be.called();
      expect(stateRepositoryMock.fetchDataContract).to.be.calledOnceWith(id);
    });
  });
});
