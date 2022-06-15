const fs = require('fs');
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

    fs.rmSync('./db/grovedb_test', { recursive: true, force: true });
  });

  describe('#create', () => {
    beforeEach(async () => {
      await store.createTree([], IdentityStoreRepository.TREE_PATH[0]);
    });

    it('should create an identity', async () => {
      const result = await repository.create(
        identity,
      );

      expect(result).to.be.instanceOf(StorageResult);
      expect(result.getOperations().length).to.be.greaterThan(0);

      const encodedIdentityResult = await store.get(
        IdentityStoreRepository.TREE_PATH.concat([identity.getId().toBuffer()]),
        IdentityStoreRepository.IDENTITY_KEY,
      );

      const [protocolVersion, rawIdentity] = decodeProtocolEntity(
        encodedIdentityResult.getValue(),
      );

      rawIdentity.protocolVersion = protocolVersion;

      const fetchedIdentity = new Identity(rawIdentity);

      expect(fetchedIdentity.toObject()).to.deep.equal(identity.toObject());
    });

    it('should store identity using transaction', async () => {
      await store.startTransaction();

      await repository.create(
        identity,
        { useTransaction: true },
      );

      const notFoundIdentityResult = await store.get(
        IdentityStoreRepository.TREE_PATH.concat([identity.getId().toBuffer()]),
        IdentityStoreRepository.IDENTITY_KEY,
        { useTransaction: false },
      );

      expect(notFoundIdentityResult.isNull()).to.be.true();

      const identityTransactionResult = await store.get(
        IdentityStoreRepository.TREE_PATH.concat([identity.getId().toBuffer()]),
        IdentityStoreRepository.IDENTITY_KEY,
        { useTransaction: true },
      );

      let [protocolVersion, rawIdentity] = decodeProtocolEntity(
        identityTransactionResult.getValue(),
      );

      rawIdentity.protocolVersion = protocolVersion;

      let fetchedIdentity = new Identity(rawIdentity);

      expect(fetchedIdentity.toObject()).to.deep.equal(identity.toObject());

      await store.commitTransaction();

      const committedIdentityResult = await store.get(
        IdentityStoreRepository.TREE_PATH.concat([identity.getId().toBuffer()]),
        IdentityStoreRepository.IDENTITY_KEY,
        { useTransaction: true },
      );

      [protocolVersion, rawIdentity] = decodeProtocolEntity(committedIdentityResult.getValue());

      rawIdentity.protocolVersion = protocolVersion;

      fetchedIdentity = new Identity(rawIdentity);

      expect(fetchedIdentity.toObject()).to.deep.equal(identity.toObject());
    });
  });

  describe('#update', () => {
    beforeEach(async () => {
      await store.createTree([], IdentityStoreRepository.TREE_PATH[0]);
    });

    it('should update identity', async () => {
      await repository.create(
        identity,
      );

      const [, publicKey] = identity.getPublicKeys();

      publicKey.setReadOnly(true);

      const result = await repository.update(
        identity,
      );

      expect(result).to.be.instanceOf(StorageResult);
      expect(result.getOperations().length).to.be.greaterThan(0);

      const encodedIdentityResult = await store.get(
        IdentityStoreRepository.TREE_PATH.concat([identity.getId().toBuffer()]),
        IdentityStoreRepository.IDENTITY_KEY,
      );

      const [protocolVersion, rawIdentity] = decodeProtocolEntity(
        encodedIdentityResult.getValue(),
      );

      rawIdentity.protocolVersion = protocolVersion;

      const fetchedIdentity = new Identity(rawIdentity);

      expect(fetchedIdentity.toObject()).to.deep.equal(identity.toObject());
    });

    it('should store identity using transaction', async () => {
      // Create identity
      await repository.create(
        identity,
      );

      await store.startTransaction();

      // Update identity
      const updatedIdentity = new Identity(identity.toObject());

      const [, publicKey] = updatedIdentity.getPublicKeys();

      publicKey.setReadOnly(true);

      await repository.update(
        updatedIdentity,
        { useTransaction: true },
      );

      const previousIdentityResult = await store.get(
        IdentityStoreRepository.TREE_PATH.concat([identity.getId().toBuffer()]),
        IdentityStoreRepository.IDENTITY_KEY,
        { useTransaction: false },
      );

      let [protocolVersion, rawIdentity] = decodeProtocolEntity(previousIdentityResult.getValue());

      rawIdentity.protocolVersion = protocolVersion;

      let fetchedIdentity = new Identity(rawIdentity);

      expect(fetchedIdentity.toObject()).to.deep.equal(identity.toObject());

      const identityTransactionResult = await store.get(
        IdentityStoreRepository.TREE_PATH.concat([identity.getId().toBuffer()]),
        IdentityStoreRepository.IDENTITY_KEY,
        { useTransaction: true },
      );

      [protocolVersion, rawIdentity] = decodeProtocolEntity(
        identityTransactionResult.getValue(),
      );

      rawIdentity.protocolVersion = protocolVersion;

      fetchedIdentity = new Identity(rawIdentity);

      expect(fetchedIdentity.toObject()).to.deep.equal(updatedIdentity.toObject());

      await store.commitTransaction();

      const committedIdentityResult = await store.get(
        IdentityStoreRepository.TREE_PATH.concat([identity.getId().toBuffer()]),
        IdentityStoreRepository.IDENTITY_KEY,
        { useTransaction: true },
      );

      [protocolVersion, rawIdentity] = decodeProtocolEntity(committedIdentityResult.getValue());

      rawIdentity.protocolVersion = protocolVersion;

      fetchedIdentity = new Identity(rawIdentity);

      expect(fetchedIdentity.toObject()).to.deep.equal(updatedIdentity.toObject());
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

      expect(result.getValue()).to.be.null();
    });

    it('should fetch an identity', async () => {
      await store.createTree(IdentityStoreRepository.TREE_PATH, identity.getId().toBuffer());

      await store.put(
        IdentityStoreRepository.TREE_PATH.concat([identity.getId().toBuffer()]),
        IdentityStoreRepository.IDENTITY_KEY,
        identity.toBuffer(),
      );

      const result = await repository.fetch(identity.getId());

      expect(result).to.be.instanceOf(StorageResult);
      expect(result.getOperations().length).to.be.greaterThan(0);

      const storedIdentity = result.getValue();

      expect(storedIdentity).to.be.an.instanceof(Identity);
      expect(storedIdentity.toObject()).to.deep.equal(identity.toObject());
    });

    it('should fetch an identity using transaction', async () => {
      await store.startTransaction();

      await store.createTree(
        IdentityStoreRepository.TREE_PATH,
        identity.getId().toBuffer(),
        { useTransaction: true },
      );

      await store.put(
        IdentityStoreRepository.TREE_PATH.concat([identity.getId().toBuffer()]),
        IdentityStoreRepository.IDENTITY_KEY,
        identity.toBuffer(),
        { useTransaction: true },
      );

      const notFoundIdentityResult = await repository.fetch(identity.getId(), {
        useTransaction: false,
      });

      expect(notFoundIdentityResult.getValue()).to.be.null();

      const transactionalIdentityResult = await repository.fetch(identity.getId(), {
        useTransaction: true,
      });

      const transactionalIdentity = transactionalIdentityResult.getValue();

      expect(transactionalIdentity).to.be.an.instanceof(Identity);
      expect(transactionalIdentity.toObject()).to.deep.equal(identity.toObject());

      await store.commitTransaction();

      const storedIdentityResult = await repository.fetch(identity.getId());

      const storedIdentity = storedIdentityResult.getValue();

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
