const fs = require('fs');
const Drive = require('@dashevo/rs-drive');
const getIdentityFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityFixture');
const Identity = require('@dashevo/dpp/lib/identity/Identity');
const decodeProtocolEntityFactory = require('@dashevo/dpp/lib/decodeProtocolEntityFactory');
const PublicKeyToIdentitiesStoreRepository = require('../../../lib/identity/PublicKeyToIdentitiesStoreRepository');
const GroveDBStore = require('../../../lib/storage/GroveDBStore');
const logger = require('../../../lib/util/noopLogger');
const StorageResult = require('../../../lib/storage/StorageResult');
const IdentityStoreRepository = require('../../../lib/identity/IdentityStoreRepository');

describe('PublicKeyToIdentitiesStoreRepository', () => {
  let rsDrive;
  let store;
  let publicKeyRepository;
  let identityRepository;
  let publicKeyHash;
  let identity;

  beforeEach(async () => {
    rsDrive = new Drive('./db/grovedb_test');
    store = new GroveDBStore(rsDrive, logger, 'blockchainStateTestStore');

    const decodeProtocolEntity = decodeProtocolEntityFactory();

    identityRepository = new IdentityStoreRepository(store, decodeProtocolEntity);

    publicKeyRepository = new PublicKeyToIdentitiesStoreRepository(store, decodeProtocolEntity);

    publicKeyHash = Buffer.alloc(20).fill(1);
    identity = getIdentityFixture();
  });

  afterEach(async () => {
    await rsDrive.close();

    fs.rmSync('./db/grovedb_test', { recursive: true, force: true });
  });

  describe('#store', () => {
    beforeEach(async () => {
      await store.createTree([], PublicKeyToIdentitiesStoreRepository.TREE_PATH[0]);
      await identityRepository.createTree();
    });

    it('should store public key to identities', async () => {
      await identityRepository.create(identity);

      const result = await publicKeyRepository.store(
        publicKeyHash,
        identity.getId(),
      );

      expect(result).to.be.instanceOf(StorageResult);
      expect(result.getOperations().length).to.be.greaterThan(0);

      const fetchedIdentityResult = await store.get(
        PublicKeyToIdentitiesStoreRepository.TREE_PATH.concat([publicKeyHash]),
        identity.getId().toBuffer(),
      );

      expect(fetchedIdentityResult).to.be.instanceOf(StorageResult);
      expect(fetchedIdentityResult.getValue()).to.be.deep.equal(identity.toBuffer());
    });

    it('should store public key to identities using transaction', async () => {
      await identityRepository.create(identity);

      await store.startTransaction();

      await publicKeyRepository.store(
        publicKeyHash,
        identity.getId(),
        { useTransaction: true },
      );

      const emptyIdentitiesResult = await store.get(
        PublicKeyToIdentitiesStoreRepository.TREE_PATH.concat([publicKeyHash]),
        identity.getId().toBuffer(),
      );

      expect(emptyIdentitiesResult).to.be.instanceOf(StorageResult);
      expect(emptyIdentitiesResult.isNull()).to.be.true();

      const transactionalIdentitiesResult = await store.get(
        PublicKeyToIdentitiesStoreRepository.TREE_PATH.concat([publicKeyHash]),
        identity.getId().toBuffer(),
        { useTransaction: true },
      );

      expect(transactionalIdentitiesResult).to.be.instanceOf(StorageResult);
      expect(transactionalIdentitiesResult.getValue()).to.be.deep.equal(identity.toBuffer());

      await store.commitTransaction();

      const committedIdentitiesResult = await store.get(
        PublicKeyToIdentitiesStoreRepository.TREE_PATH.concat([publicKeyHash]),
        identity.getId().toBuffer(),
      );

      expect(committedIdentitiesResult).to.be.instanceOf(StorageResult);
      expect(committedIdentitiesResult.getValue()).to.be.deep.equal(identity.toBuffer());
    });
  });

  describe('#fetch', () => {
    beforeEach(async () => {
      await store.createTree([], PublicKeyToIdentitiesStoreRepository.TREE_PATH[0]);
      await identityRepository.createTree();
    });

    it('should fetch empty array if public key to identities not found', async () => {
      const result = await publicKeyRepository.fetch(publicKeyHash);

      expect(result).to.be.empty();
    });

    it('should fetch an public key to identity ids map', async () => {
      await identityRepository.create(identity);

      await publicKeyRepository.store(
        publicKeyHash,
        identity.getId(),
      );

      const result = await publicKeyRepository.fetch(publicKeyHash);

      expect(result).to.be.instanceOf(StorageResult);
      expect(result.getOperations().length).to.be.greaterThan(0);

      expect(result.getValue()).to.deep.have.lengthOf(1);

      const [fetchedIdentity] = result.getValue();

      expect(fetchedIdentity).to.be.instanceOf(Identity);
      expect(fetchedIdentity).to.deep.equal(identity.toObject());
    });

    it('should fetch an public key to identities using transaction', async () => {
      await store.startTransaction();

      await identityRepository.create(identity, { useTransaction: true });

      await publicKeyRepository.store(
        publicKeyHash,
        identity.getId(),
        { useTransaction: true },
      );

      const emptyIdentitiesResult = await publicKeyRepository.fetch(publicKeyHash, {
        useTransaction: false,
      });

      expect(emptyIdentitiesResult.isEmpty()).to.be.true();

      const transactionalIdentitiesResult = await publicKeyRepository.fetch(publicKeyHash, {
        useTransaction: true,
      });

      expect(transactionalIdentitiesResult.getValue()).to.deep.equal([identity]);

      await store.commitTransaction();

      const storedIdentitiesResult = await publicKeyRepository.fetch(publicKeyHash);

      expect(storedIdentitiesResult.getValue()).to.deep.equal([identity]);
    });
  });

  describe('#fetchMany', () => {
    let publicKeyHash2;

    beforeEach(async () => {
      await store.createTree([], PublicKeyToIdentitiesStoreRepository.TREE_PATH[0]);
      await identityRepository.createTree();

      publicKeyHash2 = Buffer.alloc(20).fill(2);
    });

    it('should fetch empty array if public key to identities map not found', async () => {
      const result = await publicKeyRepository.fetchMany([publicKeyHash, publicKeyHash2]);

      expect(result).to.be.instanceOf(StorageResult);
      expect(result.getValue()).to.be.empty();
    });

    it('should fetch an public key to identities', async () => {
      await identityRepository.create(identity);

      await publicKeyRepository.store(
        publicKeyHash,
        identity.getId(),
      );

      await publicKeyRepository.store(
        publicKeyHash2,
        identity.getId(),
      );

      const result = await publicKeyRepository.fetchMany([publicKeyHash, publicKeyHash2]);

      expect(result).to.be.instanceOf(StorageResult);
      expect(result.getOperations().length).to.be.greaterThan(0);

      expect(result.getValue()).to.deep.equal([identity, identity]);
    });

    it('should fetch an public key to identities map using transaction', async () => {
      await store.startTransaction();

      await identityRepository.create(identity, { useTransaction: true });

      await publicKeyRepository.store(
        publicKeyHash,
        identity.getId(),
        { useTransaction: true },
      );

      await publicKeyRepository.store(
        publicKeyHash2,
        identity.getId(),
        { useTransaction: true },
      );

      const emptyIdentitiesResult = await publicKeyRepository.fetchMany(
        [publicKeyHash, publicKeyHash2],
        {
          useTransaction: false,
        },
      );

      expect(emptyIdentitiesResult.isEmpty()).to.be.true();

      const transactionalIdentitiesResult = await publicKeyRepository.fetchMany(
        [publicKeyHash, publicKeyHash2],
        {
          useTransaction: true,
        },
      );

      expect(transactionalIdentitiesResult.getValue()).to.deep.equal([identity, identity]);

      await store.commitTransaction();

      const storedIdentitiesResult = await publicKeyRepository.fetchMany(
        [publicKeyHash, publicKeyHash2],
      );

      expect(storedIdentitiesResult.getValue()).to.deep.equal([identity, identity]);
    });
  });

  describe('#createTree', () => {
    it('should create a tree', async () => {
      const result = await publicKeyRepository.createTree();

      expect(result).to.be.instanceOf(StorageResult);
      expect(result.getOperations().length).to.be.greaterThan(0);

      const data = await store.db.get(
        [],
        PublicKeyToIdentitiesStoreRepository.TREE_PATH[0],
      );

      expect(data).to.deep.equal({
        type: 'tree',
        value: Buffer.alloc(32),
      });
    });
  });

  describe('#prove', () => {
    let publicKeyHash2;

    beforeEach(async () => {
      await store.createTree([], PublicKeyToIdentitiesStoreRepository.TREE_PATH[0]);
      await identityRepository.createTree();

      publicKeyHash2 = Buffer.alloc(20).fill(2);
    });

    it('should fetch proof if public key to identities map not found', async () => {
      const result = await publicKeyRepository.proveMany([publicKeyHash, publicKeyHash2]);

      expect(result).to.be.instanceOf(StorageResult);

      expect(result.getValue()).to.be.an.instanceOf(Buffer);
      expect(result.getValue().length).to.be.greaterThan(0);
    });

    it('should return proof', async () => {
      await identityRepository.create(identity);

      await publicKeyRepository.store(
        publicKeyHash,
        identity.getId(),
      );

      await publicKeyRepository.store(
        publicKeyHash2,
        identity.getId(),
      );

      const result = await publicKeyRepository.proveMany([publicKeyHash, publicKeyHash2]);

      expect(result).to.be.instanceOf(StorageResult);

      expect(result.getOperations().length).to.be.greaterThan(0);

      expect(result.getValue()).to.be.an.instanceOf(Buffer);
      expect(result.getValue().length).to.be.greaterThan(0);
    });

    // TODO: Enable when transactions will be supported for queries with proofs
    it.skip('should return proof map using transaction', async () => {
      await store.startTransaction();

      await identityRepository.create(identity, { useTransaction: true });

      await publicKeyRepository.store(
        publicKeyHash,
        identity.getId(),
        { useTransaction: true },
      );

      await publicKeyRepository.store(
        publicKeyHash2,
        identity.getId(),
        { useTransaction: true },
      );

      // Should return proof of non-existence
      let result = await publicKeyRepository.proveMany([publicKeyHash, publicKeyHash2]);

      expect(result).to.be.instanceOf(StorageResult);

      expect(result.getValue()).to.be.an.instanceOf(Buffer);
      expect(result.getValue().length).to.be.greaterThan(0);

      // Should return proof of existence
      result = await publicKeyRepository.proveMany(
        [publicKeyHash, publicKeyHash2],
        { useTransaction: true },
      );

      expect(result).to.be.instanceOf(StorageResult);

      expect(result.getOperations().length).to.be.greaterThan(0);

      expect(result.getValue()).to.be.an.instanceOf(Buffer);
      expect(result.getValue().length).to.be.greaterThan(0);

      await store.commitTransaction();

      // Should return proof of existence
      result = await publicKeyRepository.proveMany([publicKeyHash, publicKeyHash2]);

      expect(result).to.be.instanceOf(StorageResult);

      expect(result.getOperations().length).to.be.greaterThan(0);

      expect(result.getValue()).to.be.an.instanceOf(Buffer);
      expect(result.getValue().length).to.be.greaterThan(0);
    });
  });
});
