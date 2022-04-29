const rimraf = require('rimraf');
const Drive = require('@dashevo/rs-drive');
const decodeProtocolEntityFactory = require('@dashevo/dpp/lib/decodeProtocolEntityFactory');
const getIdentityFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityFixture');
const Identity = require('@dashevo/dpp/lib/identity/Identity');
const GroveDBStore = require('../../../lib/storage/GroveDBStore');
const IdentityStoreRepository = require('../../../lib/identity/IdentityStoreRepository');
const logger = require('../../../lib/util/noopLogger');
const StorageResult = require('../../../lib/storage/StorageResult');

describe('IdentityStoreRepository', () => {
  let rsDrive;
  let store;
  let repository;
  let decodeProtocolEntity;
  let identity;

  beforeEach(async () => {
    rsDrive = new Drive('./db/grovedb_test');
    store = new GroveDBStore(rsDrive, logger, 'blockchainStateTestStore');

    decodeProtocolEntity = decodeProtocolEntityFactory();

    repository = new IdentityStoreRepository(store, decodeProtocolEntity);
    identity = getIdentityFixture();
  });

  afterEach(async () => {
    await rsDrive.close();
    rimraf.sync('./db/grovedb_test');
  });

  describe('#store', () => {
    beforeEach(async () => {
      await store.createTree([], IdentityStoreRepository.TREE_PATH[0]);
    });

    it('should store identity', async () => {
      const result = await repository.store(
        identity,
      );

      expect(result).to.be.instanceOf(StorageResult);
      expect(result.getOperations().length).to.be.greaterThan(0);

      const encodedIdentityResult = await store.get(
        IdentityStoreRepository.TREE_PATH,
        identity.getId().toBuffer(),
      );

      const [protocolVersion, rawIdentity] = decodeProtocolEntity(
        encodedIdentityResult.getResult(),
      );

      rawIdentity.protocolVersion = protocolVersion;

      const fetchedIdentity = new Identity(rawIdentity);

      expect(fetchedIdentity.toObject()).to.deep.equal(identity.toObject());
    });

    it('should store identity using transaction', async () => {
      await store.startTransaction();

      await repository.store(
        identity,
        true,
      );

      const notFoundIdentityResult = await store.get(
        IdentityStoreRepository.TREE_PATH,
        identity.getId().toBuffer(),
        { useTransaction: false },
      );

      expect(notFoundIdentityResult.getResult()).to.be.null();

      const identityTransactionResult = await store.get(
        IdentityStoreRepository.TREE_PATH,
        identity.getId().toBuffer(),
        { useTransaction: true },
      );

      let [protocolVersion, rawIdentity] = decodeProtocolEntity(
        identityTransactionResult.getResult(),
      );

      rawIdentity.protocolVersion = protocolVersion;

      let fetchedIdentity = new Identity(rawIdentity);

      expect(fetchedIdentity.toObject()).to.deep.equal(identity.toObject());

      await store.commitTransaction();

      const committedIdentityResult = await store.get(
        IdentityStoreRepository.TREE_PATH,
        identity.getId().toBuffer(),
        { useTransaction: true },
      );

      [protocolVersion, rawIdentity] = decodeProtocolEntity(committedIdentityResult.getResult());

      rawIdentity.protocolVersion = protocolVersion;

      fetchedIdentity = new Identity(rawIdentity);

      expect(fetchedIdentity.toObject()).to.deep.equal(identity.toObject());
    });
  });

  describe('#fetch', () => {
    beforeEach(async () => {
      await store.createTree([], IdentityStoreRepository.TREE_PATH[0]);
    });

    it('should fetch null if identity not found', async () => {
      const result = await repository.fetch(identity.getId());

      expect(result).to.be.instanceOf(StorageResult);
      expect(result.getOperations().length).to.be.greaterThan(0);

      expect(result.getResult()).to.be.null();
    });

    it('should fetch an identity', async () => {
      await store.put(
        IdentityStoreRepository.TREE_PATH,
        identity.getId().toBuffer(),
        identity.toBuffer(),
      );

      const result = await repository.fetch(identity.getId());

      expect(result).to.be.instanceOf(StorageResult);
      expect(result.getOperations().length).to.be.greaterThan(0);

      const storedIdentity = result.getResult();

      expect(storedIdentity).to.be.an.instanceof(Identity);
      expect(storedIdentity.toObject()).to.deep.equal(identity.toObject());
    });

    it('should fetch an identity using transaction', async () => {
      await store.startTransaction();

      await store.put(
        IdentityStoreRepository.TREE_PATH,
        identity.getId().toBuffer(),
        identity.toBuffer(),
        { useTransaction: true },
      );

      const notFoundIdentityResult = await repository.fetch(identity.getId(), false);

      expect(notFoundIdentityResult.getResult()).to.be.null();

      const transactionalIdentityResult = await repository.fetch(identity.getId(), true);

      const transactionalIdentity = transactionalIdentityResult.getResult();

      expect(transactionalIdentity).to.be.an.instanceof(Identity);
      expect(transactionalIdentity.toObject()).to.deep.equal(identity.toObject());

      await store.commitTransaction();

      const storedIdentityResult = await repository.fetch(identity.getId());

      const storedIdentity = storedIdentityResult.getResult();

      expect(storedIdentity).to.be.an.instanceof(Identity);
      expect(storedIdentity.toObject()).to.deep.equal(identity.toObject());
    });
  });

  describe('#createTree', () => {
    it('should create a tree', async () => {
      const result = await repository.createTree();

      expect(result).to.be.instanceOf(StorageResult);
      expect(result.getOperations().length).to.be.greaterThan(0);

      const data = await store.db.get(
        [],
        IdentityStoreRepository.TREE_PATH[0],
      );

      expect(data).to.deep.equal({
        type: 'tree',
        value: Buffer.alloc(32),
      });
    });
  });
});
