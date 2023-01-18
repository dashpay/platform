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

  beforeEach(async () => {
    rsDrive = new Drive('./db/grovedb_test', {
      dataContractsGlobalCacheSize: 500,
      dataContractsBlockCacheSize: 500,
    });

    await rsDrive.createInitialStateStructure();

    store = new GroveDBStore(rsDrive, logger);

    decodeProtocolEntity = decodeProtocolEntityFactory();

    repository = new IdentityStoreRepository(store, decodeProtocolEntity);
    identity = getIdentityFixture();

    blockInfo = new BlockInfo(1, 1, Date.now());
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

  describe('#addToBalance', () => {
    beforeEach(async () => {
      await repository.create(
        identity,
        blockInfo,
      );
    });

    it('should add to balance', async () => {
      const amount = 100;

      const result = await repository.addToBalance(
        identity.getId(),
        amount,
        blockInfo,
      );

      expect(result).to.be.instanceOf(StorageResult);
      expect(result.getOperations().length).to.equal(1);

      const fetchedIdentity = await rsDrive.fetchIdentity(
        identity.getId(),
      );

      expect(fetchedIdentity.getBalance()).to.equal(identity.getBalance() + amount);
    });

    it('should add to balance using transaction', async () => {
      await store.startTransaction();

      const amount = 100;

      await repository.addToBalance(
        identity.getId(),
        amount,
        blockInfo,
        { useTransaction: true },
      );

      const previousIdentity = await rsDrive.fetchIdentity(
        identity.getId(),
      );

      expect(previousIdentity.getBalance()).to.equal(identity.getBalance());

      const transactionalIdentity = await rsDrive.fetchIdentity(
        identity.getId(),
        true,
      );

      expect(transactionalIdentity.getBalance()).to.equal(identity.getBalance() + amount);

      await store.commitTransaction();

      const commitedIdentity = await rsDrive.fetchIdentity(
        identity.getId(),
      );

      expect(commitedIdentity.getBalance()).to.equal(identity.getBalance() + amount);
    });
  });

  describe('#removeFromBalance', () => {
    beforeEach(async () => {
      await repository.create(
        identity,
        blockInfo,
      );
    });

    it('should remove from balance', async () => {
      const amount = 5;

      const result = await repository.removeFromBalance(
        identity.getId(),
        amount,
        blockInfo,
      );

      expect(result).to.be.instanceOf(StorageResult);
      expect(result.getOperations().length).to.equal(1);

      const fetchedIdentity = await rsDrive.fetchIdentity(
        identity.getId(),
      );

      expect(fetchedIdentity.getBalance()).to.equal(identity.getBalance() - amount);
    });

    it('should remove from balance using transaction', async () => {
      await store.startTransaction();

      const amount = 5;

      await repository.removeFromBalance(
        identity.getId(),
        amount,
        blockInfo,
        { useTransaction: true },
      );

      const previousIdentity = await rsDrive.fetchIdentity(
        identity.getId(),
      );

      expect(previousIdentity.getBalance()).to.equal(identity.getBalance());

      const transactionalIdentity = await rsDrive.fetchIdentity(
        identity.getId(),
        true,
      );

      expect(transactionalIdentity.getBalance()).to.equal(identity.getBalance() - amount);

      await store.commitTransaction();

      const commitedIdentity = await rsDrive.fetchIdentity(
        identity.getId(),
      );

      expect(commitedIdentity.getBalance()).to.equal(identity.getBalance() - amount);
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

  describe('#fetchByPublicKeyHash', () => {
    context('without block info', () => {
      it('should return empty array if identity with public key hash not found', async () => {
        const result = await repository.fetchByPublicKeyHash(Buffer.alloc(32));

        expect(result).to.be.instanceOf(StorageResult);
        expect(result.getOperations().length).to.equal(1);

        expect(result.getValue()).to.be.instanceOf(Array);
        expect(result.getValue()).to.be.empty();
      });

      it('should fetch an identity', async () => {
        await rsDrive.insertIdentity(identity, blockInfo);

        const publicKeyHash = identity.getPublicKeys()[0].hash();

        const result = await repository.fetchByPublicKeyHash(publicKeyHash);

        expect(result).to.be.instanceOf(StorageResult);
        expect(result.getOperations().length).to.equal(1);

        expect(result.getValue()).to.have.lengthOf(1);

        const [fetchedIdentity] = result.getValue();

        expect(fetchedIdentity).to.be.instanceOf(Identity);
        expect(fetchedIdentity).to.deep.equal(identity.toObject());
      });

      it('should fetch an identity using transaction', async () => {
        await store.startTransaction();

        await rsDrive.insertIdentity(identity, blockInfo, true);

        const publicKeyHash = identity.getPublicKeys()[0].hash();

        const emptyIdentitiesResult = await repository.fetchByPublicKeyHash(publicKeyHash, {
          useTransaction: true,
        });

        expect(emptyIdentitiesResult.isEmpty()).to.be.true();

        const transactionalIdentitiesResult = await repository.fetchByPublicKeyHash(publicKeyHash, {
          useTransaction: false,
        });

        expect(transactionalIdentitiesResult.getValue()).to.have.lengthOf(1);

        const [transactionalIdentity] = transactionalIdentitiesResult.getValue();

        expect(transactionalIdentity).to.be.instanceOf(Identity);
        expect(transactionalIdentity).to.deep.equal(identity.toObject());

        await store.commitTransaction();

        const storedIdentityResult = await repository.fetchByPublicKeyHash(publicKeyHash);

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
