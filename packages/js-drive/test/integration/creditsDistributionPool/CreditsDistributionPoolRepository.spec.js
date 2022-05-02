const fs = require('fs');
const cbor = require('cbor');
const Drive = require('@dashevo/rs-drive');
const GroveDBStore = require('../../../lib/storage/GroveDBStore');
const CreditsDistributionPoolRepository = require('../../../lib/creditsDistributionPool/CreditsDistributionPoolRepository');
const CreditsDistributionPool = require('../../../lib/creditsDistributionPool/CreditsDistributionPool');
const logger = require('../../../lib/util/noopLogger');
const StorageResult = require('../../../lib/storage/StorageResult');

describe('CreditsDistributionPoolRepository', () => {
  let rsDrive;
  let store;
  let repository;
  let creditsDistributionPool;
  let amount;

  beforeEach(async () => {
    rsDrive = new Drive('./db/grovedb_test');
    store = new GroveDBStore(rsDrive, logger, 'creditsDistributionPoolTestStore');

    await store.createTree([], CreditsDistributionPoolRepository.PATH[0]);

    repository = new CreditsDistributionPoolRepository(store);

    amount = 42;

    creditsDistributionPool = new CreditsDistributionPool(amount);
  });

  afterEach(async () => {
    await rsDrive.close();

    fs.rmSync('./db/grovedb_test', { recursive: true });
  });

  describe('#store', () => {
    it('should store creditsDistributionPool', async () => {
      const result = await repository.store(creditsDistributionPool);

      expect(result).to.be.instanceOf(StorageResult);

      expect(result.getOperations().length).to.be.greaterThan(0);

      const storedCreditsDistributionPoolBufferResult = await store.get(
        CreditsDistributionPoolRepository.PATH,
        CreditsDistributionPoolRepository.KEY,
      );

      const storedCreditsDistributionPoolBuffer = storedCreditsDistributionPoolBufferResult
        .getValue();

      expect(storedCreditsDistributionPoolBuffer).to.be.instanceOf(Buffer);

      const storedCreditsDistributionPool = cbor.decode(
        storedCreditsDistributionPoolBuffer,
      );

      expect(storedCreditsDistributionPool.amount).to.equal(amount);
    });

    it('should store creditsDistributionPool using transaction', async () => {
      await store.startTransaction();

      const result = await repository.store(creditsDistributionPool, true);

      expect(result).to.be.instanceOf(StorageResult);

      expect(result.getOperations().length).to.be.greaterThan(0);

      const notFoundDataResult = await store.get(
        CreditsDistributionPoolRepository.PATH,
        CreditsDistributionPoolRepository.KEY,
      );

      expect(notFoundDataResult.getValue()).to.be.null();

      const dataFromTransactionResult = await store.get(
        CreditsDistributionPoolRepository.PATH,
        CreditsDistributionPoolRepository.KEY,
        { useTransaction: true },
      );

      const dataFromTransaction = cbor.decode(dataFromTransactionResult.getValue());

      expect(dataFromTransaction.amount).to.equal(amount);

      await store.commitTransaction();

      const committedDataResult = await store.get(
        CreditsDistributionPoolRepository.PATH,
        CreditsDistributionPoolRepository.KEY,
      );

      const committedData = cbor.decode(committedDataResult.getValue());

      expect(committedData.amount).to.equal(amount);
    });
  });

  describe('#fetch', () => {
    it('should fetch empty CreditsDistributionPool', async () => {
      const result = await repository.fetch();

      expect(result).to.be.instanceOf(StorageResult);

      expect(result.getOperations().length).to.be.greaterThan(0);

      const fetchedCreditsDistributionPool = result.getValue();

      expect(fetchedCreditsDistributionPool).to.be.instanceOf(
        CreditsDistributionPool,
      );

      expect(fetchedCreditsDistributionPool.getAmount()).to.equals(0);
    });

    it('should fetch stored CreditsDistributionPool', async () => {
      await store.put(
        CreditsDistributionPoolRepository.PATH,
        CreditsDistributionPoolRepository.KEY,
        cbor.encodeCanonical(
          creditsDistributionPool.toJSON(),
        ),
      );

      const result = await repository.fetch();

      expect(result).to.be.instanceOf(StorageResult);

      expect(result.getOperations().length).to.be.greaterThan(0);

      const storedCreditsDistributionPool = result.getValue();

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

      // Nothing without transaction
      const emptyResult = await repository.fetch(false);

      expect(emptyResult).to.be.instanceOf(StorageResult);

      expect(emptyResult.getOperations().length).to.be.greaterThan(0);

      const emptyPool = emptyResult.getValue();

      expect(emptyPool).to.be.instanceOf(CreditsDistributionPool);
      expect(emptyPool.getAmount()).to.equals(0);

      // Actual amount in transactions
      const transactionalResult = await repository.fetch(true);

      const transactionalPool = transactionalResult.getValue();

      expect(transactionalPool).to.be.instanceOf(CreditsDistributionPool);
      expect(transactionalPool.getAmount()).to.equals(amount);

      await store.commitTransaction();

      // Actual amount without transaction
      const committedResults = await repository.fetch(false);

      const committedPool = committedResults.getValue();

      expect(committedPool).to.be.instanceOf(CreditsDistributionPool);
      expect(committedPool.getAmount()).to.equals(amount);
    });
  });
});
