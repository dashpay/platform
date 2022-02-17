const rimraf = require('rimraf');
const Drive = require('@dashevo/rs-drive');
const decodeProtocolEntityFactory = require('@dashevo/dpp/lib/decodeProtocolEntityFactory');
const getIdentityFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityFixture');
const Identity = require('@dashevo/dpp/lib/identity/Identity');
const GroveDBStore = require('../../../lib/storage/GroveDBStore');
const IdentityStoreRepository = require('../../../lib/identity/IdentityStoreRepository');

describe('IdentityStoreRepository', () => {
  let rsDrive;
  let store;
  let repository;
  let decodeProtocolEntity;
  let identity;

  beforeEach(async () => {
    rsDrive = new Drive('./db/grovedb_test');
    store = new GroveDBStore(rsDrive, 'blockchainStateTestStore');

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

      expect(result).to.be.equal(repository);

      const encodedIdentity = await store.get(
        IdentityStoreRepository.TREE_PATH,
        identity.getId().toBuffer(),
      );

      const [protocolVersion, rawIdentity] = decodeProtocolEntity(encodedIdentity);

      rawIdentity.protocolVersion = protocolVersion;

      expect(identity.toJSON()).to.deep.equal(new Identity(rawIdentity).toJSON());
    });

    it('should store identity using transaction', async () => {
      await store.startTransaction();

      await repository.store(
        identity,
        true,
      );

      const notFoundIdentity = await store.get(
        IdentityStoreRepository.TREE_PATH,
        identity.getId().toBuffer(),
        { useTransaction: false },
      );

      expect(notFoundIdentity).to.be.null();

      const identityTransaction = await store.get(
        IdentityStoreRepository.TREE_PATH,
        identity.getId().toBuffer(),
        { useTransaction: true },
      );

      let [protocolVersion, rawIdentity] = decodeProtocolEntity(identityTransaction);

      rawIdentity.protocolVersion = protocolVersion;

      expect(identity.toJSON()).to.deep.equal(new Identity(rawIdentity).toJSON());

      await store.commitTransaction();

      const committedIdentity = await store.get(
        IdentityStoreRepository.TREE_PATH,
        identity.getId().toBuffer(),
        { useTransaction: true },
      );

      [protocolVersion, rawIdentity] = decodeProtocolEntity(committedIdentity);

      rawIdentity.protocolVersion = protocolVersion;

      expect(identity.toJSON()).to.deep.equal(new Identity(rawIdentity).toJSON());
    });
  });

  describe('#fetch', () => {
    beforeEach(async () => {
      await store.createTree([], IdentityStoreRepository.TREE_PATH[0]);
    });

    it('should fetch null if identity not found', async () => {
      const result = await repository.fetch(identity.getId());

      expect(result).to.be.null();
    });

    it('should fetch an identity', async () => {
      await store.put(
        IdentityStoreRepository.TREE_PATH,
        identity.getId().toBuffer(),
        identity.toBuffer(),
      );

      const storedIdentity = await repository.fetch(identity.getId());

      expect(storedIdentity).to.be.an.instanceof(Identity);
      expect(storedIdentity.toJSON()).to.deep.equal(identity.toJSON());
    });

    it('should fetch an identity using transaction', async () => {
      await store.startTransaction();

      await store.put(
        IdentityStoreRepository.TREE_PATH,
        identity.getId().toBuffer(),
        identity.toBuffer(),
        { useTransaction: true },
      );

      const notFoundIdentity = await repository.fetch(identity.getId(), false);

      expect(notFoundIdentity).to.be.null();

      const transactionalIdentity = await repository.fetch(identity.getId(), true);

      expect(transactionalIdentity).to.be.an.instanceof(Identity);
      expect(transactionalIdentity.toJSON()).to.deep.equal(identity.toJSON());

      await store.commitTransaction();

      const storedIdentity = await repository.fetch(identity.getId());

      expect(storedIdentity).to.be.an.instanceof(Identity);
      expect(storedIdentity.toJSON()).to.deep.equal(identity.toJSON());
    });
  });

  describe('#createTree', () => {
    it('should create a tree', async () => {
      const result = await repository.createTree();

      expect(result).to.equal(repository);

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
