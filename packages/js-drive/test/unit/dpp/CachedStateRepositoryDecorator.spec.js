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
      storePublicKeyIdentityId: this.sinon.stub(),
      fetchPublicKeyIdentityId: this.sinon.stub(),
      fetchLatestPlatformBlockHeader: this.sinon.stub(),
      fetchIdentityIdsByPublicKeyHashes: this.sinon.stub(),
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

  describe('#storePublicKeyIdentityId', () => {
    it('should store identity id and public key hash pair to repository', async () => {
      const [firstPublicKey] = identity.getPublicKeys();

      await cachedStateRepository.storePublicKeyIdentityId(
        firstPublicKey.hash(), identity.getId(),
      );

      expect(stateRepositoryMock.storePublicKeyIdentityId).to.be.calledOnceWithExactly(
        firstPublicKey.hash(), identity.getId(),
      );
    });
  });

  describe('#fetchPublicKeyIdentityId', () => {
    it('should fetch identity id by public key hash from repository', async () => {
      const [firstPublicKey] = identity.getPublicKeys();

      stateRepositoryMock.fetchPublicKeyIdentityId.resolves(identity.getId());

      const result = await cachedStateRepository.fetchPublicKeyIdentityId(
        firstPublicKey.hash(),
      );

      expect(stateRepositoryMock.fetchPublicKeyIdentityId).to.be.calledOnceWithExactly(
        firstPublicKey.hash(),
      );
      expect(result).to.deep.equal(identity.getId());
    });
  });

  describe('#fetchIdentityIdsByPublicKeyHashes', () => {
    it('should fetch identity id and public key hash pairs map from repository', async () => {
      const publicKeys = identity.getPublicKeys();

      stateRepositoryMock.fetchIdentityIdsByPublicKeyHashes.resolves({
        [publicKeys[0].hash()]: identity.getId(),
        [publicKeys[1].hash()]: identity.getId(),
      });

      const result = await cachedStateRepository.fetchIdentityIdsByPublicKeyHashes(
        publicKeys.map((pk) => pk.hash()),
      );

      expect(stateRepositoryMock.fetchIdentityIdsByPublicKeyHashes).to.be.calledOnceWithExactly(
        publicKeys.map((pk) => pk.hash()),
      );
      expect(result).to.deep.equal({
        [publicKeys[0].hash()]: identity.getId(),
        [publicKeys[1].hash()]: identity.getId(),
      });
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

  describe('#fetchLatestPlatformBlockHeader', () => {
    it('should fetch latest platform block header from state repository', async () => {
      const header = {
        height: 10,
        time: {
          seconds: Math.ceil(new Date().getTime() / 1000),
        },
      };

      stateRepositoryMock.fetchLatestPlatformBlockHeader.resolves(header);

      const result = await cachedStateRepository.fetchLatestPlatformBlockHeader(id);

      expect(result).to.deep.equal(header);
      expect(stateRepositoryMock.fetchLatestPlatformBlockHeader).to.be.calledOnce();
    });
  });
});
