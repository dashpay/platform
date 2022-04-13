const cbor = require('cbor');

const CreditsDistributionPoolRepository = require('../../../lib/creditsDistributionPool/CreditsDistributionPoolRepository');
const CreditsDistributionPool = require('../../../lib/creditsDistributionPool/CreditsDistributionPool');

describe('CreditsDistributionPoolRepository', () => {
  let externalStoreMock;
  let repository;
  let creditsDistributionPool;
  let initialCreditsDistributionPool;

  beforeEach(function beforeEach() {
    externalStoreMock = {
      put: this.sinon.stub(),
      get: this.sinon.stub(),
    };

    repository = new CreditsDistributionPoolRepository(externalStoreMock);

    initialCreditsDistributionPool = 42;

    creditsDistributionPool = new CreditsDistributionPool(initialCreditsDistributionPool);
  });

  describe('#store', () => {
    it('should store credits distribution pool', async () => {
      const repositoryInstance = await repository.store(creditsDistributionPool, false);
      expect(repositoryInstance).to.equal(repository);

      expect(externalStoreMock.put).to.be.calledOnceWithExactly(
        CreditsDistributionPoolRepository.PATH,
        CreditsDistributionPoolRepository.KEY,
        cbor.encodeCanonical(creditsDistributionPool.toJSON()),
        { useTransaction: false },
      );
    });
  });

  describe('#fetch', () => {
    it('should return empty credits distribution pool if it is not stored', async () => {
      externalStoreMock.get.returns(null);

      const result = await repository.fetch(true);

      expect(result).to.be.instanceOf(CreditsDistributionPool);
      expect(result.getAmount()).to.be.a('number');
      expect(result.getAmount()).to.equal(0);

      expect(externalStoreMock.get).to.be.calledOnceWithExactly(
        CreditsDistributionPoolRepository.PATH,
        CreditsDistributionPoolRepository.KEY,
        { useTransaction: true },
      );
    });

    it('should return stored chain info', async () => {
      const storedStateBuffer = cbor.encode(creditsDistributionPool.toJSON());

      externalStoreMock.get.returns(storedStateBuffer);

      const result = await repository.fetch(false);

      expect(result).to.be.instanceOf(CreditsDistributionPool);
      expect(result.getAmount()).to.be.a('number');
      expect(result.getAmount()).to.equal(initialCreditsDistributionPool);

      expect(externalStoreMock.get).to.be.calledOnceWithExactly(
        CreditsDistributionPoolRepository.PATH,
        CreditsDistributionPoolRepository.KEY,
        { useTransaction: false },
      );
    });
  });
});
