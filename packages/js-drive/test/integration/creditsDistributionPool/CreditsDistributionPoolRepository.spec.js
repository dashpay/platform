const rimraf = require('rimraf');
const cbor = require('cbor');
const Drive = require('@dashevo/rs-drive');
const GroveDBStore = require('../../../lib/storage/GroveDBStore');
const CreditsDistributionPoolRepository = require('../../../lib/creditsDistributionPool/CreditsDistributionPoolRepository');
const CreditsDistributionPool = require('../../../lib/creditsDistributionPool/CreditsDistributionPool');

describe('CreditsDistributionPoolRepository', () => {
  let rsDrive;
  let store;
  let repository;
  let creditsDistributionPool;
  let amount;

  beforeEach(async () => {
    rsDrive = new Drive('./db/grovedb_test');
    store = new GroveDBStore(rsDrive, 'creditsDistributionPoolTestStore');

    await store.createTree([], CreditsDistributionPoolRepository.PATH[0]);

    repository = new CreditsDistributionPoolRepository(store);

    amount = 42;

    creditsDistributionPool = new CreditsDistributionPool(amount);
  });

  afterEach(async () => {
    await rsDrive.close();
    rimraf.sync('./db/grovedb_test');
  });

  describe('#store', () => {
    it('should store creditsDistributionPool', async () => {
      const result = await repository.store(creditsDistributionPool);

      expect(result).to.equal(repository);

      const storedCreditsDistributionPoolBuffer = await store.get(
        CreditsDistributionPoolRepository.PATH,
        CreditsDistributionPoolRepository.KEY,
      );

      expect(storedCreditsDistributionPoolBuffer).to.be.instanceOf(Buffer);
      const storedCreditsDistributionPool = cbor.decode(storedCreditsDistributionPoolBuffer);

      expect(storedCreditsDistributionPool.amount).to.equal(amount);
    });

    it('should store creditsDistributionPool using transaction', async () => {
      await store.startTransaction();

      const result = await repository.store(creditsDistributionPool, true);

      expect(result).to.equal(repository);

      const notFoundData = await store.get(
        CreditsDistributionPoolRepository.PATH,
        CreditsDistributionPoolRepository.KEY,
      );

      expect(notFoundData).to.be.null();

      const dataFromTransaction = await store.get(
        CreditsDistributionPoolRepository.PATH,
        CreditsDistributionPoolRepository.KEY,
        { useTransaction: true },
      );

      expect(cbor.decode(dataFromTransaction).amount).to.equal(amount);

      await store.commitTransaction();

      const committedData = await store.get(
        CreditsDistributionPoolRepository.PATH,
        CreditsDistributionPoolRepository.KEY,
      );

      expect(cbor.decode(committedData).amount).to.equal(amount);
    });
  });

  describe('#fetch', () => {
    it('should fetch empty CreditsDistributionPool', async () => {
      const storedCreditsDistributionPool = await repository.fetch();

      expect(storedCreditsDistributionPool).to.be.instanceOf(CreditsDistributionPool);
      expect(storedCreditsDistributionPool.getAmount()).to.equals(0);
    });

    it('should fetch stored CreditsDistributionPool', async () => {
      await store.put(
        CreditsDistributionPoolRepository.PATH,
        CreditsDistributionPoolRepository.KEY,
        cbor.encodeCanonical(
          creditsDistributionPool.toJSON(),
        ),
      );

      const storedCreditsDistributionPool = await repository.fetch();
      expect(storedCreditsDistributionPool).to.be.instanceOf(CreditsDistributionPool);
      expect(storedCreditsDistributionPool.getAmount()).to.equals(amount);
    });

    it('should fetch stored CreditsDistributionPool using transaction', async () => {
      await store.startTransaction();

      await store.put(
        CreditsDistributionPoolRepository.PATH,
        CreditsDistributionPoolRepository.KEY,
        cbor.encodeCanonical(
          creditsDistributionPool.toJSON(),
        ),
        { useTransaction: true },
      );

      const emptyCreditsDistributionPool = await repository.fetch(false);

      expect(emptyCreditsDistributionPool).to.be.instanceOf(CreditsDistributionPool);
      expect(emptyCreditsDistributionPool.getAmount()).to.equals(0);

      const transactionalCreditsDistributionPool = await repository.fetch(true);

      expect(transactionalCreditsDistributionPool).to.be.instanceOf(CreditsDistributionPool);
      expect(transactionalCreditsDistributionPool.getAmount()).to.equals(amount);

      await store.commitTransaction();

      const committedCreditsDistributionPool = await repository.fetch(false);

      expect(committedCreditsDistributionPool).to.be.instanceOf(CreditsDistributionPool);
      expect(committedCreditsDistributionPool.getAmount()).to.equals(amount);
    });
  });
});
