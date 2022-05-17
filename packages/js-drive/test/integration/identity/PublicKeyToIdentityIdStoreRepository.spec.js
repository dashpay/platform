const rimraf = require('rimraf');
const Drive = require('@dashevo/rs-drive');
const getIdentityFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityFixture');
const Identifier = require('@dashevo/dpp/lib/Identifier');
const cbor = require('cbor');
const PublicKeyToIdentityIdStoreRepository = require('../../../lib/identity/PublicKeyToIdentityIdStoreRepository');
const GroveDBStore = require('../../../lib/storage/GroveDBStore');
const logger = require('../../../lib/util/noopLogger');
const StorageResult = require('../../../lib/storage/StorageResult');

describe('PublicKeyToIdentityIdStoreRepository', () => {
  let rsDrive;
  let store;
  let repository;
  let publicKeyHash;
  let identity;

  beforeEach(async () => {
    rsDrive = new Drive('./db/grovedb_test');
    store = new GroveDBStore(rsDrive, logger, 'blockchainStateTestStore');

    repository = new PublicKeyToIdentityIdStoreRepository(store);

    publicKeyHash = Buffer.alloc(20).fill(1);
    identity = getIdentityFixture();
  });

  afterEach(async () => {
    await rsDrive.close();
    rimraf.sync('./db/grovedb_test');
  });

  describe('#store', () => {
    beforeEach(async () => {
      await store.createTree([], PublicKeyToIdentityIdStoreRepository.TREE_PATH[0]);
    });

    it('should store public key to identity ids map', async () => {
      const result = await repository.store(
        publicKeyHash,
        identity.getId(),
      );

      expect(result).to.be.instanceOf(StorageResult);
      expect(result.getOperations().length).to.be.greaterThan(0);

      const identityIdsSerializedResult = await store.get(
        PublicKeyToIdentityIdStoreRepository.TREE_PATH,
        publicKeyHash,
      );

      const identityIds = cbor.decode(identityIdsSerializedResult.getValue());

      expect(identityIds).to.have.lengthOf(1);

      const identifier = new Identifier(identityIds[0]);

      expect(identifier).to.deep.equal(identity.getId());
    });

    it('should store public key to identity ids map using transaction', async () => {
      await store.startTransaction();

      await repository.store(
        publicKeyHash,
        identity.getId(),
        { useTransaction: true },
      );

      const emptyIdsResult = await store.get(
        PublicKeyToIdentityIdStoreRepository.TREE_PATH,
        publicKeyHash,
      );

      expect(emptyIdsResult.getValue()).to.be.null();

      const transactionalIdsEncodedResult = await store.get(
        PublicKeyToIdentityIdStoreRepository.TREE_PATH,
        publicKeyHash,
        { useTransaction: true },
      );

      const transactionalIds = cbor.decode(transactionalIdsEncodedResult.getValue());

      expect(transactionalIds).to.have.lengthOf(1);

      let identifier = new Identifier(transactionalIds[0]);

      expect(identifier).to.deep.equal(identity.getId());

      await store.commitTransaction();

      const committedIdsEncodedResult = await store.get(
        PublicKeyToIdentityIdStoreRepository.TREE_PATH,
        publicKeyHash,
      );

      const committedIds = cbor.decode(committedIdsEncodedResult.getValue());

      expect(committedIds).to.have.lengthOf(1);

      identifier = new Identifier(committedIds[0]);

      expect(identifier).to.deep.equal(identity.getId());
    });
  });

  describe('#fetch', () => {
    beforeEach(async () => {
      await store.createTree([], PublicKeyToIdentityIdStoreRepository.TREE_PATH[0]);
    });

    it('should fetch empty array if public key to identity ids map not found', async () => {
      const result = await repository.fetch(publicKeyHash);

      expect(result).to.be.empty();
    });

    it('should fetch an public key to identity ids map', async () => {
      const identityIds = [identity.getId().toBuffer()];

      await store.put(
        PublicKeyToIdentityIdStoreRepository.TREE_PATH,
        publicKeyHash,
        cbor.encode(identityIds),
      );

      const result = await repository.fetch(publicKeyHash);

      expect(result).to.be.instanceOf(StorageResult);
      expect(result.getOperations().length).to.be.greaterThan(0);

      const fetchedIdentityIds = result.getValue();

      expect(fetchedIdentityIds).to.deep.have.lengthOf(1);

      const identifier = new Identifier(fetchedIdentityIds[0]);

      expect(identifier).to.deep.equal(identity.getId());
    });

    it('should fetch an public key to identity ids map using transaction', async () => {
      const identityIds = [identity.getId()];

      await store.startTransaction();

      await store.put(
        PublicKeyToIdentityIdStoreRepository.TREE_PATH,
        publicKeyHash,
        cbor.encode(identityIds.map(((id) => id.toBuffer()))),
        { useTransaction: true },
      );

      const emptyIdsResult = await repository.fetch(publicKeyHash, {
        useTransaction: false,
      });

      expect(emptyIdsResult.getValue()).to.be.empty();

      const transactionalIdsResult = await repository.fetch(publicKeyHash, {
        useTransaction: true,
      });

      expect(transactionalIdsResult.getValue()).to.deep.equal(identityIds);

      await store.commitTransaction();

      const storedIdsResult = await repository.fetch(publicKeyHash);

      expect(storedIdsResult.getValue()).to.deep.equal(identityIds);
    });
  });

  describe('#fetchBuffer', () => {
    beforeEach(async () => {
      await store.createTree([], PublicKeyToIdentityIdStoreRepository.TREE_PATH[0]);
    });

    it('should fetch serialized identity ids by public key hash', async () => {
      const identityIds = [identity.getId().toBuffer()];

      await store.put(
        PublicKeyToIdentityIdStoreRepository.TREE_PATH,
        publicKeyHash,
        cbor.encode(identityIds),
      );

      const result = await repository.fetchBuffer(publicKeyHash);

      expect(result).to.be.instanceOf(StorageResult);
      expect(result.getOperations().length).to.be.greaterThan(0);

      const storedIdsEncoded = result.getValue();

      const storedIds = cbor.decode(storedIdsEncoded).map((id) => new Identifier(id));

      expect(storedIds).to.deep.have.lengthOf(1);

      const identifier = new Identifier(storedIds[0]);

      expect(identifier).to.deep.equal(identity.getId());
    });

    it('should fetch serialized identity ids by public key hash in transaction', async () => {
      const identityIds = [identity.getId()];

      await store.startTransaction();

      await store.put(
        PublicKeyToIdentityIdStoreRepository.TREE_PATH,
        publicKeyHash,
        cbor.encode(identityIds.map(((id) => id.toBuffer()))),
        { useTransaction: true },
      );

      const emptyIdsResult = await repository.fetchBuffer(publicKeyHash, {
        useTransaction: false,
      });

      expect(emptyIdsResult.getValue()).to.be.null();

      const transactionalIdsEncodedResult = await repository.fetchBuffer(publicKeyHash, {
        useTransaction: true,
      });

      const transactionalIdsEncoded = transactionalIdsEncodedResult.getValue();

      const transactionalIds = cbor.decode(transactionalIdsEncoded).map((id) => new Identifier(id));
      expect(transactionalIds).to.deep.equal(identityIds);

      await store.commitTransaction();

      const storedIdsEncodedResult = await repository.fetchBuffer(publicKeyHash);

      const storedIdsEncoded = storedIdsEncodedResult.getValue();

      const storedIds = cbor.decode(storedIdsEncoded).map((id) => new Identifier(id));

      expect(storedIds).to.deep.equal(identityIds);
    });
  });

  describe('#createTree', () => {
    it('should create a tree', async () => {
      const result = await repository.createTree();

      expect(result).to.be.instanceOf(StorageResult);
      expect(result.getOperations().length).to.be.greaterThan(0);

      const data = await store.db.get(
        [],
        PublicKeyToIdentityIdStoreRepository.TREE_PATH[0],
      );

      expect(data).to.deep.equal({
        type: 'tree',
        value: Buffer.alloc(32),
      });
    });
  });
});
