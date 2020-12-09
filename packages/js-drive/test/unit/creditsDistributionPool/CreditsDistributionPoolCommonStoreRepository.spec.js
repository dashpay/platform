const cbor = require('cbor');

const CreditsDistributionPoolCommonStoreRepository = require('../../../lib/creditsDistributionPool/CreditsDistributionPoolCommonStoreRepository');
const CreditsDistributionPool = require('../../../lib/creditsDistributionPool/CreditsDistributionPool');

describe('ChainInfoExternalStoreRepository', () => {
  let externalStoreMock;
  let repository;
  let creditsDistributionPool;
  let initialCreditsDistributionPool;
  let transactionMock;

  beforeEach(function beforeEach() {
    externalStoreMock = {
      put: this.sinon.stub(),
      get: this.sinon.stub(),
    };

    repository = new CreditsDistributionPoolCommonStoreRepository(externalStoreMock);

    initialCreditsDistributionPool = 42;

    creditsDistributionPool = new CreditsDistributionPool(initialCreditsDistributionPool);

    transactionMock = 'transaction';
  });

  describe('#store', () => {
    it('should store credits distribution pool', async () => {
      const repositoryInstance = await repository.store(creditsDistributionPool, transactionMock);
      expect(repositoryInstance).to.equal(repository);

      expect(externalStoreMock.put).to.be.calledOnceWithExactly(
        CreditsDistributionPoolCommonStoreRepository.COMMON_STORE_KEY_NAME,
        cbor.encodeCanonical(creditsDistributionPool.toJSON()),
        transactionMock,
      );
    });
  });

  describe('#fetch', () => {
    it('should return empty credits distribution pool if it is not stored', async () => {
      externalStoreMock.get.returns(null);

      const result = await repository.fetch(transactionMock);

      expect(result).to.be.instanceOf(CreditsDistributionPool);
      expect(result.getAmount()).to.be.a('number');
      expect(result.getAmount()).to.equal(0);

      expect(externalStoreMock.get).to.be.calledOnceWithExactly(
        CreditsDistributionPoolCommonStoreRepository.COMMON_STORE_KEY_NAME,
        transactionMock,
      );
    });

    it('should return stored chain info', async () => {
      const storedStateBuffer = cbor.encode(creditsDistributionPool.toJSON());

      externalStoreMock.get.returns(storedStateBuffer);

      const result = await repository.fetch(transactionMock);

      expect(result).to.be.instanceOf(CreditsDistributionPool);
      expect(result.getAmount()).to.be.a('number');
      expect(result.getAmount()).to.equal(initialCreditsDistributionPool);

      expect(externalStoreMock.get).to.be.calledOnceWithExactly(
        CreditsDistributionPoolCommonStoreRepository.COMMON_STORE_KEY_NAME,
        transactionMock,
      );
    });
  });
});
