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
const BlockInfo = require('../../../lib/blockExecution/BlockInfo');

describe('IdentityStoreRepository', () => {
  let rsDrive;
  let store;
  let repository;
  let decodeProtocolEntity;
  let identity;
  let blockInfo;
  let publicKeyHashes;

  beforeEach(async () => {
    rsDrive = new Drive('./db/grovedb_test', {
      drive: {
        dataContractsGlobalCacheSize: 500,
        dataContractsBlockCacheSize: 500,
      },
      core: {
        rpc: {
          url: '127.0.0.1',
          username: '',
          password: '',
        },
      },
    });

    await rsDrive.createInitialStateStructure();

    store = new GroveDBStore(rsDrive, logger);

    decodeProtocolEntity = decodeProtocolEntityFactory();

    repository = new IdentityStoreRepository(store, decodeProtocolEntity);
    identity = getIdentityFixture();

    blockInfo = new BlockInfo(1, 1, Date.now());

    publicKeyHashes = identity.getPublicKeys().map((k) => k.hash());
  });

  afterEach(async () => {
    await rsDrive.close();

    fs.rmSync('./db/grovedb_test', { recursive: true, force: true });
  });

  describe('#create', () => {
    it('should create an identity', async () => {
      const result = await repository.create(
        identity,
        blockInfo,
      );

      expect(result).to.be.instanceOf(StorageResult);
      expect(result.getOperations().length).to.equal(1);

      const fetchedResult = await rsDrive.fetchIdentity(
        identity.getId(),
      );

      expect(fetchedResult.toObject()).to.be.deep.equal(identity.toObject());
    });

    it('should store identity using transaction', async () => {
      await store.startTransaction();

      await repository.create(
        identity,
        blockInfo,
        { useTransaction: true },
      );

      const notFoundIdentity = await rsDrive.fetchIdentity(
        identity.getId(),
      );

      expect(notFoundIdentity).to.be.null();

      const identityTransaction = await rsDrive.fetchIdentity(
        identity.getId(),
        true,
      );

      expect(identityTransaction.toObject()).to.deep.equal(identity.toObject());

      await store.commitTransaction();

      const committedIdentity = await rsDrive.fetchIdentity(
        identity.getId(),
      );

      expect(committedIdentity.toObject()).to.deep.equal(identity.toObject());
    });
  });

  describe('#updateRevision', () => {
    beforeEach(async () => {
      await repository.create(
        identity,
        blockInfo,
      );
    });

    it('should update revision', async () => {
      const revision = 2;

      const result = await repository.updateRevision(
        identity.getId(),
        revision,
        blockInfo,
      );

      expect(result).to.be.instanceOf(StorageResult);
      expect(result.getOperations().length).to.equal(1);

      const fetchedIdentity = await rsDrive.fetchIdentity(
        identity.getId(),
      );

      expect(fetchedIdentity.getRevision()).to.equal(revision);
    });

    it('should remove from balance using transaction', async () => {
      await store.startTransaction();

      const revision = 2;

      await repository.updateRevision(
        identity.getId(),
        revision,
        blockInfo,
        { useTransaction: true },
      );

      const previousIdentity = await rsDrive.fetchIdentity(
        identity.getId(),
      );

      expect(previousIdentity.getRevision()).to.equal(identity.getRevision());

      const transactionalIdentity = await rsDrive.fetchIdentity(
        identity.getId(),
        true,
      );

      expect(transactionalIdentity.getRevision()).to.equal(revision);

      await store.commitTransaction();

      const commitedIdentity = await rsDrive.fetchIdentity(
        identity.getId(),
      );

      expect(commitedIdentity.getRevision()).to.equal(revision);
    });
  });

  describe('#fetch', () => {
    context('without block info', () => {
      it('should fetch null if identity not found', async () => {
        const result = await repository.fetch(identity.getId());

        expect(result).to.be.instanceOf(StorageResult);
        expect(result.getOperations().length).to.equal(0);

        expect(result.getValue()).to.be.null();
      });

      it('should fetch an identity', async () => {
        await rsDrive.insertIdentity(identity, blockInfo);

        const result = await repository.fetch(identity.getId());

        expect(result).to.be.instanceOf(StorageResult);
        expect(result.getOperations().length).to.equal(0);

        const storedIdentity = result.getValue();

        expect(storedIdentity).to.be.an.instanceof(Identity);
        expect(storedIdentity.toObject()).to.deep.equal(identity.toObject());
      });

      it('should fetch an identity using transaction', async () => {
        await store.startTransaction();

        await rsDrive.insertIdentity(identity, blockInfo, true);

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

    context('with block info', () => {
      it('should fetch null if identity not found', async () => {
        const result = await repository.fetch(identity.getId(), { blockInfo });

        expect(result).to.be.instanceOf(StorageResult);
        expect(result.getOperations().length).to.equal(1);

        expect(result.getValue()).to.be.null();
      });

      it('should fetch an identity', async () => {
        await rsDrive.insertIdentity(identity, blockInfo);

        const result = await repository.fetch(identity.getId(), { blockInfo });

        expect(result).to.be.instanceOf(StorageResult);
        expect(result.getOperations().length).to.equal(1);

        const storedIdentity = result.getValue();

        expect(storedIdentity).to.be.an.instanceof(Identity);
        expect(storedIdentity.toObject()).to.deep.equal(identity.toObject());
      });

      it('should fetch an identity using transaction', async () => {
        await store.startTransaction();

        await rsDrive.insertIdentity(identity, blockInfo, true);

        const notFoundIdentityResult = await repository.fetch(identity.getId(), {
          blockInfo,
          useTransaction: false,
        });

        expect(notFoundIdentityResult.getValue()).to.be.null();

        const transactionalIdentityResult = await repository.fetch(identity.getId(), {
          blockInfo,
          useTransaction: true,
        });

        const transactionalIdentity = transactionalIdentityResult.getValue();

        expect(transactionalIdentity).to.be.an.instanceof(Identity);
        expect(transactionalIdentity.toObject()).to.deep.equal(identity.toObject());

        await store.commitTransaction();

        const storedIdentityResult = await repository.fetch(identity.getId(), {
          blockInfo,
        });

        const storedIdentity = storedIdentityResult.getValue();

        expect(storedIdentity).to.be.an.instanceof(Identity);
        expect(storedIdentity.toObject()).to.deep.equal(identity.toObject());
      });
    });
  });

  describe('#fetchByPublicKeyHashes', () => {
    it('should fetch an identities by public key hashes', async () => {
      await rsDrive.insertIdentity(identity, blockInfo);

      const result = await repository.fetchManyByPublicKeyHashes(
        publicKeyHashes.concat([Buffer.alloc(20)]),
      );

      expect(result).to.be.instanceOf(StorageResult);
      expect(result.getOperations().length).to.equal(0);

      const fetchedIdentities = result.getValue();

      for (let i = 0; i < identity.getPublicKeys().length; i++) {
        const fetchedIdentity = fetchedIdentities[i];

        expect(fetchedIdentity).to.be.instanceOf(Identity);
        expect(fetchedIdentity).to.deep.equal(identity.toObject());
      }
    });
  });

  describe('#proveManyByPublicKeyHashes', () => {
    it('should fetch proof if public key to identities map not found', async () => {
      const result = await repository.proveManyByPublicKeyHashes([
        Buffer.alloc(20, 1),
        Buffer.alloc(20, 2),
      ]);

      expect(result).to.be.instanceOf(StorageResult);

      expect(result.getValue()).to.be.an.instanceOf(Buffer);
      expect(result.getValue().length).to.be.greaterThan(0);
    });

    it('should return proof', async () => {
      await repository.create(identity, blockInfo);

      const result = await repository.proveManyByPublicKeyHashes(publicKeyHashes);

      expect(result).to.be.instanceOf(StorageResult);

      expect(result.getOperations().length).to.equal(0);

      expect(result.getValue()).to.be.an.instanceOf(Buffer);
      expect(result.getValue().length).to.be.greaterThan(0);
    });

    // TODO: Enable when transactions will be supported for queries with proofs
    it.skip('should return proof map using transaction', async () => {
      await store.startTransaction();

      await repository.create(identity, blockInfo, { useTransaction: true });

      // Should return proof of non-existence
      let result = await repository.proveManyByPublicKeyHashes(publicKeyHashes);

      expect(result).to.be.instanceOf(StorageResult);

      expect(result.getValue()).to.be.an.instanceOf(Buffer);
      expect(result.getValue().length).to.be.greaterThan(0);

      // Should return proof of existence
      result = await repository.proveManyByPublicKeyHashes(
        publicKeyHashes,
        { useTransaction: true },
      );

      expect(result).to.be.instanceOf(StorageResult);

      expect(result.getOperations().length).to.equal(0);

      expect(result.getValue()).to.be.an.instanceOf(Buffer);
      expect(result.getValue().length).to.be.greaterThan(0);

      await store.commitTransaction();

      // Should return proof of existence
      result = await repository.proveManyByPublicKeyHashes(publicKeyHashes);

      expect(result).to.be.instanceOf(StorageResult);

      expect(result.getOperations().length).to.equal(0);

      expect(result.getValue()).to.be.an.instanceOf(Buffer);
      expect(result.getValue().length).to.be.greaterThan(0);
    });
  });

  describe('#prove', () => {
    it('should return prove if identity does not exist', async () => {
      const result = await repository.prove(identity.getId());

      expect(result).to.be.instanceOf(StorageResult);
      expect(result.getOperations().length).to.equal(0);

      const proof = result.getValue();

      expect(proof).to.be.an.instanceof(Buffer);
      expect(proof.length).to.be.greaterThan(0);
    });

    it('should return proof', async () => {
      await repository.create(identity, blockInfo);

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
      // Set correct but unique public key data
      const data = Buffer.from(identity.getPublicKeys()[0].getData());
      data[data.length - 1] = 2;

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
            data,
          },
        ],
        balance: 10,
        revision: 0,
      });
    });

    it('should return proof if identity does not exist', async () => {
      await repository.create(identity, blockInfo);

      const result = await repository.proveMany([identity.getId(), identity2.getId()]);

      expect(result).to.be.instanceOf(StorageResult);
      expect(result.getOperations().length).to.equal(0);

      const proof = result.getValue();

      expect(proof).to.be.an.instanceof(Buffer);
      expect(proof.length).to.be.greaterThan(0);
    });

    it('should return proof', async () => {
      await repository.create(identity, blockInfo);
      await repository.create(identity2, blockInfo);

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
