const fs = require('fs');
const Drive = require('@dashevo/rs-drive');
const { FeeResult } = require('@dashevo/rs-drive');
const decodeProtocolEntityFactory = require('@dashevo/dpp/lib/decodeProtocolEntityFactory');
const getIdentityFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityFixture');
const Identity = require('@dashevo/dpp/lib/identity/Identity');
const GroveDBStore = require('../../../lib/storage/GroveDBStore');
const IdentityBalanceStoreRepository = require('../../../lib/identity/IdentityBalanceStoreRepository');
const IdentityStoreRepository = require('../../../lib/identity/IdentityStoreRepository');
const logger = require('../../../lib/util/noopLogger');
const StorageResult = require('../../../lib/storage/StorageResult');
const BlockInfo = require('../../../lib/blockExecution/BlockInfo');

describe('IdentityStoreRepository', () => {
  let rsDrive;
  let store;
  let balanceRepository;
  let identityRepository;
  let decodeProtocolEntity;
  let identity;
  let blockInfo;

  beforeEach(async () => {
    rsDrive = new Drive('./db/grovedb_test', {
      dataContractsGlobalCacheSize: 500,
      dataContractsBlockCacheSize: 500,
    });

    await rsDrive.createInitialStateStructure();

    store = new GroveDBStore(rsDrive, logger);

    decodeProtocolEntity = decodeProtocolEntityFactory();

    balanceRepository = new IdentityBalanceStoreRepository(store, decodeProtocolEntity);
    identityRepository = new IdentityStoreRepository(store, decodeProtocolEntity);
    identity = getIdentityFixture();

    blockInfo = new BlockInfo(1, 1, Date.now());
  });

  afterEach(async () => {
    await rsDrive.close();

    fs.rmSync('./db/grovedb_test', { recursive: true, force: true });
  });

  describe('#add', () => {
    beforeEach(async () => {
      await identityRepository.create(
        identity,
        blockInfo,
      );
    });

    it('should add to balance', async () => {
      const amount = 100;

      const result = await balanceRepository.add(
        identity.getId(),
        amount,
        blockInfo,
      );

      expect(result).to.be.instanceOf(StorageResult);
      expect(result.getOperations().length).to.equal(1);

      const fetchedIdentityResult = await identityRepository.fetch(
        identity.getId(),
      );

      const fetchedIdentity = fetchedIdentityResult.getValue();

      expect(fetchedIdentity.getBalance()).to.equal(identity.getBalance() + amount);
    });

    it('should add to balance using transaction', async () => {
      await store.startTransaction();

      const amount = 100;

      await balanceRepository.add(
        identity.getId(),
        amount,
        blockInfo,
        { useTransaction: true },
      );

      const previousIdentityResult = await identityRepository.fetch(
        identity.getId(),
      );

      const previousIdentity = previousIdentityResult.getValue();

      expect(previousIdentity.getBalance()).to.equal(identity.getBalance());

      const transactionalIdentityResult = await identityRepository.fetch(
        identity.getId(),
        { useTransaction: true },
      );

      const transactionalIdentity = transactionalIdentityResult.getValue();

      expect(transactionalIdentity.getBalance()).to.equal(identity.getBalance() + amount);

      await store.commitTransaction();

      const committedIdentityResult = await identityRepository.fetch(
        identity.getId(),
      );

      const committedIdentity = committedIdentityResult.getValue();

      expect(committedIdentity.getBalance()).to.equal(identity.getBalance() + amount);
    });
  });

  describe('#applyFees', () => {
    beforeEach(async () => {
      identity.setBalance(10000);

      await identityRepository.create(
        identity,
        blockInfo,
      );
    });

    it('should apply fees to balance', async () => {
      const feeResult = FeeResult.create(1000, 100, []);

      const result = await balanceRepository.applyFees(
        identity.getId(),
        feeResult,
      );

      expect(result).to.be.instanceOf(StorageResult);
      expect(result.getOperations().length).to.equal(0);

      expect(result.getValue()).to.be.instanceOf(FeeResult);

      const fetchedIdentityResult = await identityRepository.fetch(
        identity.getId(),
      );

      const fetchedIdentity = fetchedIdentityResult.getValue();

      expect(fetchedIdentity.getBalance()).to.equal(
        identity.getBalance() - feeResult.storageFee - feeResult.processingFee,
      );
    });

    it('should add to balance using transaction', async () => {
      await store.startTransaction();

      const feeResult = FeeResult.create(1000, 100, []);

      await balanceRepository.applyFees(
        identity.getId(),
        feeResult,
        { useTransaction: true },
      );

      const previousIdentityResult = await identityRepository.fetch(
        identity.getId(),
      );

      const previousIdentity = previousIdentityResult.getValue();

      expect(previousIdentity.getBalance()).to.equal(identity.getBalance());

      const transactionalIdentityResult = await identityRepository.fetch(
        identity.getId(),
        { useTransaction: true },
      );

      const transactionalIdentity = transactionalIdentityResult.getValue();

      expect(transactionalIdentity.getBalance()).to.equal(
        identity.getBalance() - feeResult.storageFee - feeResult.processingFee,
      );

      await store.commitTransaction();

      const committedIdentityResult = await identityRepository.fetch(
        identity.getId(),
      );

      const committedIdentity = committedIdentityResult.getValue();

      expect(committedIdentity.getBalance()).to.equal(
        identity.getBalance() - feeResult.storageFee - feeResult.processingFee,
      );
    });
  });

  describe('#remove', () => {
    beforeEach(async () => {
      await identityRepository.create(
        identity,
        blockInfo,
      );
    });

    it('should remove from balance', async () => {
      const amount = 5;

      const result = await balanceRepository.remove(
        identity.getId(),
        amount,
        blockInfo,
      );

      expect(result).to.be.instanceOf(StorageResult);
      expect(result.getOperations().length).to.equal(1);

      const fetchedIdentityResult = await identityRepository.fetch(
        identity.getId(),
      );

      const fetchedIdentity = fetchedIdentityResult.getValue();

      expect(fetchedIdentity.getBalance()).to.equal(identity.getBalance() - amount);
    });

    it('should remove from balance using transaction', async () => {
      await store.startTransaction();

      const amount = 5;

      await balanceRepository.remove(
        identity.getId(),
        amount,
        blockInfo,
        { useTransaction: true },
      );

      const previousIdentityResult = await identityRepository.fetch(
        identity.getId(),
      );

      const previousIdentity = previousIdentityResult.getValue();

      expect(previousIdentity.getBalance()).to.equal(identity.getBalance());

      const transactionalIdentityResult = await identityRepository.fetch(
        identity.getId(),
        { useTransaction: true },
      );

      const transactionalIdentity = transactionalIdentityResult.getValue();

      expect(transactionalIdentity.getBalance()).to.equal(identity.getBalance() - amount);

      await store.commitTransaction();

      const committedIdentityResult = await identityRepository.fetch(
        identity.getId(),
      );

      const committedIdentity = committedIdentityResult.getValue();

      expect(committedIdentity.getBalance()).to.equal(identity.getBalance() - amount);
    });
  });

  describe('#fetch', () => {
    context('without block info', () => {
      it('should fetch null if identity not found', async () => {
        const result = await balanceRepository.fetch(identity.getId());

        expect(result).to.be.instanceOf(StorageResult);
        expect(result.getOperations().length).to.equal(0);

        expect(result.getValue()).to.be.null();
      });

      it('should fetch balance', async () => {
        await identityRepository.create(identity, blockInfo);

        const result = await balanceRepository.fetch(identity.getId());

        expect(result).to.be.instanceOf(StorageResult);
        expect(result.getOperations().length).to.equal(0);

        const balance = result.getValue();

        expect(balance).to.equals(identity.getBalance());
      });

      it('should fetch an identity using transaction', async () => {
        await store.startTransaction();

        await identityRepository.create(identity, blockInfo, {
          useTransaction: true,
        });

        const notFoundBalanceResult = await balanceRepository.fetch(identity.getId(), {
          useTransaction: false,
        });

        expect(notFoundBalanceResult.getValue()).to.be.null();

        const transactionalBalanceResult = await balanceRepository.fetch(identity.getId(), {
          useTransaction: true,
        });

        const transactionalBalance = transactionalBalanceResult.getValue();

        expect(transactionalBalance).to.equals(identity.getBalance());

        await store.commitTransaction();

        const storedBalanceResult = await balanceRepository.fetch(identity.getId());

        const storedBalance = storedBalanceResult.getValue();

        expect(storedBalance).to.equals(identity.getBalance());
      });
    });

    context('with block info', () => {
      it('should fetch null if identity not found', async () => {
        const result = await balanceRepository.fetch(identity.getId(), { blockInfo });

        expect(result).to.be.instanceOf(StorageResult);
        expect(result.getOperations().length).to.equal(1);

        expect(result.getValue()).to.be.null();
      });

      it('should fetch an identity', async () => {
        await identityRepository.create(identity, blockInfo);

        const result = await balanceRepository.fetch(identity.getId(), { blockInfo });

        expect(result).to.be.instanceOf(StorageResult);
        expect(result.getOperations().length).to.equal(1);

        const storedBalance = result.getValue();

        expect(storedBalance).to.equals(identity.getBalance());
      });

      it('should fetch an identity using transaction', async () => {
        await store.startTransaction();

        await identityRepository.create(identity, blockInfo, {
          useTransaction: true,
        });

        const notFoundBalanceResult = await balanceRepository.fetch(identity.getId(), {
          blockInfo,
          useTransaction: false,
        });

        expect(notFoundBalanceResult.getValue()).to.be.null();

        const transactionalBalanceResult = await balanceRepository.fetch(identity.getId(), {
          blockInfo,
          useTransaction: true,
        });

        const transactionalBalance = transactionalBalanceResult.getValue();

        expect(transactionalBalance).to.equals(identity.getBalance());

        await store.commitTransaction();

        const storedBalanceResult = await balanceRepository.fetch(identity.getId(), {
          blockInfo,
        });

        const storedBalance = storedBalanceResult.getValue();

        expect(storedBalance).to.equals(identity.getBalance());
      });
    });
  });

  describe('#fetchWithDebt', () => {
    it('should fetch null if identity not found', async () => {
      const result = await balanceRepository.fetchWithDebt(identity.getId(), blockInfo);

      expect(result).to.be.instanceOf(StorageResult);
      expect(result.getOperations().length).to.equal(1);

      expect(result.getValue()).to.be.null();
    });

    it('should fetch an identity', async () => {
      await identityRepository.create(identity, blockInfo);

      const result = await balanceRepository.fetchWithDebt(identity.getId(), blockInfo);

      expect(result).to.be.instanceOf(StorageResult);
      expect(result.getOperations().length).to.equal(1);

      const storedBalance = result.getValue();

      expect(storedBalance).to.equals(identity.getBalance());
    });

    it('should fetch an identity using transaction', async () => {
      await store.startTransaction();

      await identityRepository.create(identity, blockInfo, {
        useTransaction: true,
      });

      const notFoundBalanceResult = await balanceRepository.fetchWithDebt(
        identity.getId(),
        blockInfo,
        {
          useTransaction: false,
        },
      );

      expect(notFoundBalanceResult.getValue()).to.be.null();

      const transactionalBalanceResult = await balanceRepository.fetchWithDebt(
        identity.getId(),
        blockInfo,
        {
          useTransaction: true,
        },
      );

      const transactionalBalance = transactionalBalanceResult.getValue();

      expect(transactionalBalance).to.equals(identity.getBalance());

      await store.commitTransaction();

      const storedBalanceResult = await balanceRepository.fetchWithDebt(
        identity.getId(),
        blockInfo,
      );

      const storedBalance = storedBalanceResult.getValue();

      expect(storedBalance).to.equals(identity.getBalance());
    });
  });
});
