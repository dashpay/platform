const fs = require('fs');
const Drive = require('@dashevo/rs-drive');
const decodeProtocolEntityFactory = require('@dashevo/dpp/lib/decodeProtocolEntityFactory');
const getIdentityFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityFixture');
const Identity = require('@dashevo/dpp/lib/identity/Identity');
const generateRandomIdentifier = require('@dashevo/dpp/lib/test/utils/generateRandomIdentifier');
const IdentityPublicKey = require('@dashevo/dpp/lib/identity/IdentityPublicKey');
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
    rsDrive = new Drive('./db/grovedb_test', {
      drive: {
        dataContractsGlobalCacheSize: 500,
        dataContractsBlockCacheSize: 500,
      },
      core: {
        rpcUrl: '127.0.0.1',
        rpcUsername: '',
        rpcPassword: '',
      },
    });

    store = new GroveDBStore(rsDrive, logger);

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
      expect(result.getOperations().length).to.equal(0);

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
        IdentityStoreRepository.TREE_PATH,
        identity.getId().toBuffer(),
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
      expect(result.getOperations().length).to.equal(0);

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
      expect(result.getOperations().length).to.equal(0);

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
      expect(result.getOperations().length).to.equal(0);

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

  describe('#prove', () => {
    beforeEach(async () => {
      await store.createTree([], IdentityStoreRepository.TREE_PATH[0]);
    });

    it('should return prove if identity does not exist', async () => {
      const result = await repository.prove(identity.getId());

      expect(result).to.be.instanceOf(StorageResult);
      expect(result.getOperations().length).to.equal(0);

      const proof = result.getValue();

      expect(proof).to.be.an.instanceof(Buffer);
      expect(proof.length).to.be.greaterThan(0);
    });

    it('should return proof', async () => {
      await store.createTree(IdentityStoreRepository.TREE_PATH, identity.getId().toBuffer());

      await store.put(
        IdentityStoreRepository.TREE_PATH.concat([identity.getId().toBuffer()]),
        IdentityStoreRepository.IDENTITY_KEY,
        identity.toBuffer(),
      );

      const result = await repository.prove(identity.getId());

      expect(result).to.be.instanceOf(StorageResult);
      expect(result.getOperations().length).to.equal(0);

      const proof = result.getValue();

      expect(proof).to.be.an.instanceof(Buffer);
      expect(proof.length).to.be.greaterThan(0);
    });

    // TODO enable this test when we support transactions
    it.skip('should return proof using transaction', async () => {
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

      const notFoundProof = await repository.prove(identity.getId(), {
        useTransaction: false,
      });

      expect(notFoundProof.getValue()).to.be.null();

      const transactionalIdentityResult = await repository.prove(identity.getId(), {
        useTransaction: true,
      });

      const transactionalProof = transactionalIdentityResult.getValue();

      expect(transactionalProof).to.be.an.instanceof(Buffer);
      expect(transactionalProof.length).to.be.greaterThan(0);

      await store.commitTransaction();

      const storedIdentityResult = await repository.prove(identity.getId());

      const storedProof = storedIdentityResult.getValue();

      expect(storedProof).to.be.an.instanceof(Buffer);
      expect(storedProof.length).to.be.greaterThan(0);
    });
  });

  describe('#proveMany', () => {
    let identity2;

    beforeEach(async () => {
      identity2 = new Identity({
        protocolVersion: 1,
        id: generateRandomIdentifier().toBuffer(),
        publicKeys: [
          {
            id: 0,
            type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
            purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
            securityLevel: IdentityPublicKey.SECURITY_LEVELS.MASTER,
            readOnly: false,
            data: Buffer.alloc(48).fill(255),
          },
        ],
        balance: 10,
        revision: 0,
      });

      await store.createTree([], IdentityStoreRepository.TREE_PATH[0]);
    });

    it('should return proof if identity does not exist', async () => {
      // Create only first identity
      await store.createTree(IdentityStoreRepository.TREE_PATH, identity.getId().toBuffer());

      await store.put(
        IdentityStoreRepository.TREE_PATH.concat([identity.getId().toBuffer()]),
        IdentityStoreRepository.IDENTITY_KEY,
        identity.toBuffer(),
      );

      const result = await repository.proveMany([identity.getId(), identity2.getId()]);

      expect(result).to.be.instanceOf(StorageResult);
      expect(result.getOperations().length).to.equal(0);

      const proof = result.getValue();

      expect(proof).to.be.an.instanceof(Buffer);
      expect(proof.length).to.be.greaterThan(0);
    });

    it('should return proof', async () => {
      await store.createTree(IdentityStoreRepository.TREE_PATH, identity.getId().toBuffer());

      await store.put(
        IdentityStoreRepository.TREE_PATH.concat([identity.getId().toBuffer()]),
        IdentityStoreRepository.IDENTITY_KEY,
        identity.toBuffer(),
      );

      const result = await repository.proveMany([identity.getId(), identity2.getId()]);

      expect(result).to.be.instanceOf(StorageResult);
      expect(result.getOperations().length).to.equal(0);

      const proof = result.getValue();

      expect(proof).to.be.an.instanceof(Buffer);
      expect(proof.length).to.be.greaterThan(0);
    });

    // TODO enable this test when we support transactions
    it.skip('should return proof using transaction', async () => {
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

      const notFoundProof = await repository.proveMany([identity.getId(), identity2.getId()], {
        useTransaction: false,
      });

      expect(notFoundProof.getValue()).to.be.null();

      const transactionalIdentityResult = await repository.proveMany(
        [identity.getId(), identity2.getId()],
        { useTransaction: true },
      );

      const transactionalProof = transactionalIdentityResult.getValue();

      expect(transactionalProof).to.be.an.instanceof(Buffer);
      expect(transactionalProof.length).to.be.greaterThan(0);

      await store.commitTransaction();

      const storedIdentityResult = await repository.proveMany(
        [identity.getId(), identity2.getId()],
      );

      const storedProof = storedIdentityResult.getValue();

      expect(storedProof).to.be.an.instanceof(Buffer);
      expect(storedProof.length).to.be.greaterThan(0);
    });
  });
});
