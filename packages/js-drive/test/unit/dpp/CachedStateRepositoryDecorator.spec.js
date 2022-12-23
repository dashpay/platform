const getDocumentsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentsFixture');
const getIdentityFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityFixture');
const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const createStateRepositoryMock = require('@dashevo/dpp/lib/test/mocks/createStateRepositoryMock');

const CachedStateRepositoryDecorator = require('../../../lib/dpp/CachedStateRepositoryDecorator');

describe('CachedStateRepositoryDecorator', () => {
  let stateRepositoryMock;
  let cachedStateRepository;
  let id;
  let identity;
  let documents;
  let dataContract;

  beforeEach(function beforeEach() {
    id = 'id';
    identity = getIdentityFixture();
    documents = getDocumentsFixture();
    dataContract = getDataContractFixture();

    stateRepositoryMock = createStateRepositoryMock(this.sinon);

    cachedStateRepository = new CachedStateRepositoryDecorator(
      stateRepositoryMock,
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

  describe('#createIdentity', () => {
    it('should store identity to repository', async () => {
      await cachedStateRepository.createIdentity(identity);

      expect(stateRepositoryMock.createIdentity).to.be.calledOnceWith(identity);
    });
  });

  describe('#updateIdentity', () => {
    it('should store identity to repository', async () => {
      await cachedStateRepository.updateIdentity(identity);

      expect(stateRepositoryMock.updateIdentity).to.be.calledOnceWith(identity);
    });
  });

  describe('#storeIdentityPublicKeyHashes', () => {
    it('should store identity id and public key hashes to repository', async () => {
      const publicKeyHashes = identity.getPublicKeys().map((pk) => pk.hash());

      await cachedStateRepository.storeIdentityPublicKeyHashes(
        identity.getId(), publicKeyHashes,
      );

      expect(stateRepositoryMock.storeIdentityPublicKeyHashes).to.be.calledOnceWithExactly(
        identity.getId(), publicKeyHashes, undefined,
      );
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
        undefined,
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

  describe('#createDocument', () => {
    it('should create document in repository', async () => {
      const [document] = documents;

      await cachedStateRepository.createDocument(document);

      expect(stateRepositoryMock.createDocument).to.be.calledOnceWith(document);
    });
  });

  describe('#updateDocument', () => {
    it('should update document in repository', async () => {
      const [document] = documents;

      await cachedStateRepository.updateDocument(document);

      expect(stateRepositoryMock.updateDocument).to.be.calledOnceWith(document);
    });
  });

  describe('#removeDocument', () => {
    it('should delete document from repository', async () => {
      const type = 'documentType';

      await cachedStateRepository.removeDocument(dataContract, type, id);

      expect(stateRepositoryMock.removeDocument).to.be.calledOnceWith(dataContract, type, id);
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
    it('should fetch data contract from state repository if it is not present in cache', async () => {
      stateRepositoryMock.fetchDataContract.resolves(dataContract);

      const result = await cachedStateRepository.fetchDataContract(id);

      expect(result).to.equal(dataContract);
      expect(stateRepositoryMock.fetchDataContract).to.be.calledOnceWith(id);
    });
  });

  describe('#fetchLatestPlatformBlockHeight', () => {
    it('should fetch latest platform height from state repository', async () => {
      stateRepositoryMock.fetchLatestPlatformBlockHeight.resolves(10);

      const result = await cachedStateRepository.fetchLatestPlatformBlockHeight(id);

      expect(result).to.equal(10);
      expect(stateRepositoryMock.fetchLatestPlatformBlockHeight).to.be.calledOnce();
    });
  });

  describe('#fetchLatestPlatformBlockTime', () => {
    it('should fetch latest platform block time from state repository', async () => {
      const timeMs = Date.now();

      stateRepositoryMock.fetchLatestPlatformBlockTime.returns(timeMs);

      const result = await cachedStateRepository.fetchLatestPlatformBlockTime();

      expect(result).to.deep.equal(timeMs);
      expect(stateRepositoryMock.fetchLatestPlatformBlockTime).to.be.calledOnce();
    });
  });

  describe('#fetchLatestPlatformCoreChainLockedHeight', () => {
    it('should fetch latest platform core chain locked height from state repository', async () => {
      const height = 42;

      stateRepositoryMock.fetchLatestPlatformCoreChainLockedHeight.resolves(height);

      const result = await cachedStateRepository.fetchLatestPlatformCoreChainLockedHeight(id);

      expect(result).to.deep.equal(height);
      expect(stateRepositoryMock.fetchLatestPlatformCoreChainLockedHeight).to.be.calledOnce();
    });
  });

  describe('#fetchLatestWithdrawalTransactionIndex', () => {
    it('should call fetchLatestWithdrawalTransactionIndex', async () => {
      stateRepositoryMock.fetchLatestWithdrawalTransactionIndex.resolves(42);

      const result = await cachedStateRepository.fetchLatestWithdrawalTransactionIndex();

      expect(result).to.equal(42);
      expect(
        stateRepositoryMock.fetchLatestWithdrawalTransactionIndex,
      ).to.have.been.calledOnce();
    });
  });

  describe('#enqueueWithdrawalTransaction', () => {
    it('should call enqueueWithdrawalTransaction', async () => {
      const index = 42;
      const transactionBytes = Buffer.alloc(32, 1);

      await cachedStateRepository.enqueueWithdrawalTransaction(
        index, transactionBytes,
      );

      expect(
        stateRepositoryMock.enqueueWithdrawalTransaction,
      ).to.have.been.calledOnceWithExactly(
        index,
        transactionBytes,
      );
    });
  });
});
