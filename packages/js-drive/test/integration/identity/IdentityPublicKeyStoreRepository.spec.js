const fs = require('fs');
const Drive = require('@dashevo/rs-drive');
const getIdentityFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityFixture');
const decodeProtocolEntityFactory = require('@dashevo/dpp/lib/decodeProtocolEntityFactory');
const IdentityPublicKeyStoreRepository = require('../../../lib/identity/IdentityPublicKeyStoreRepository');
const GroveDBStore = require('../../../lib/storage/GroveDBStore');
const logger = require('../../../lib/util/noopLogger');
const StorageResult = require('../../../lib/storage/StorageResult');
const IdentityStoreRepository = require('../../../lib/identity/IdentityStoreRepository');
const BlockInfo = require('../../../lib/blockExecution/BlockInfo');

describe('IdentityPublicKeyStoreRepository', () => {
  let rsDrive;
  let store;
  let publicKeyRepository;
  let identityRepository;
  let identity;
  let blockInfo;

  beforeEach(async () => {
    rsDrive = new Drive('./db/grovedb_test', {
      dataContractsGlobalCacheSize: 500,
      dataContractsBlockCacheSize: 500,
    });

    store = new GroveDBStore(rsDrive, logger, {
      dataContractsGlobalCacheSize: 500,
      dataContractsBlockCacheSize: 500,
    });

    await rsDrive.createInitialStateStructure();

    const decodeProtocolEntity = decodeProtocolEntityFactory();

    identityRepository = new IdentityStoreRepository(store, decodeProtocolEntity);

    publicKeyRepository = new IdentityPublicKeyStoreRepository(store, decodeProtocolEntity);

    identity = getIdentityFixture();

    blockInfo = new BlockInfo(1, 1, Date.now());
  });

  afterEach(async () => {
    await rsDrive.close();

    fs.rmSync('./db/grovedb_test', { recursive: true, force: true });
  });

  describe('#add', () => {
    it('should add public keys to identity', async () => {
      const publicKey = identity.getPublicKeys().pop();

      await identityRepository.create(identity, blockInfo);

      const result = await publicKeyRepository.add(
        identity.getId(),
        [publicKey],
        blockInfo,
      );

      expect(result).to.be.instanceOf(StorageResult);
      expect(result.getOperations().length).to.equal(1);

      const fetchedIdentityResult = await identityRepository.fetch(identity.getId());

      const fetchedIdentity = fetchedIdentityResult.getValue();

      identity.getPublicKeys().push(publicKey);

      expect(fetchedIdentity.toObject()).to.deep.equals(identity.toObject());
    });

    it('should store public key to identities using transaction', async () => {
      const publicKey = identity.getPublicKeys().pop();

      await identityRepository.create(identity, blockInfo);

      await store.startTransaction();

      await publicKeyRepository.add(
        identity.getId(),
        [publicKey],
        blockInfo,
        { useTransaction: true },
      );

      const noKeyIdentityResult = await identityRepository.fetch(identity.getId());

      const noKeyIdentity = noKeyIdentityResult.getValue();

      expect(noKeyIdentity.toObject()).to.deep.equals(identity.toObject());

      const transactionalIdentityResult = await identityRepository.fetch(identity.getId(), {
        useTransaction: true,
      });

      const transactionalIdentity = transactionalIdentityResult.getValue();

      identity.getPublicKeys().push(publicKey);

      expect(transactionalIdentity.toObject()).to.deep.equals(identity.toObject());

      await store.commitTransaction();

      const committedIdentityResult = await identityRepository.fetch(identity.getId());

      const committedIdentity = committedIdentityResult.getValue();

      expect(committedIdentity.toObject()).to.deep.equals(identity.toObject());
    });
  });

  describe('#disable', () => {
    it('should disable public keys in identity', async () => {
      await identityRepository.create(identity, blockInfo);

      const result = await publicKeyRepository.disable(
        identity.getId(),
        [0, 1],
        Date.now(),
        blockInfo,
      );

      expect(result).to.be.instanceOf(StorageResult);
      expect(result.getOperations().length).to.equal(1);

      const fetchedIdentityResult = await identityRepository.fetch(identity.getId());

      const fetchedIdentity = fetchedIdentityResult.getValue();

      expect(fetchedIdentity.toObject()).to.not.deep.equals(identity.toObject());
    });

    it('should store public key to identities using transaction', async () => {
      await identityRepository.create(identity, blockInfo);

      await store.startTransaction();

      await publicKeyRepository.disable(
        identity.getId(),
        [0, 1],
        Date.now(),
        blockInfo,
        { useTransaction: true },
      );

      const noChangeIdentityResult = await identityRepository.fetch(identity.getId());

      const noChangeIdentity = noChangeIdentityResult.getValue();

      expect(noChangeIdentity.toObject()).to.deep.equals(identity.toObject());

      const transactionalIdentityResult = await identityRepository.fetch(identity.getId(), {
        useTransaction: true,
      });

      const transactionalIdentity = transactionalIdentityResult.getValue();

      expect(transactionalIdentity.toObject()).to.not.deep.equals(identity.toObject());

      await store.commitTransaction();

      const committedIdentityResult = await identityRepository.fetch(identity.getId());

      const committedIdentity = committedIdentityResult.getValue();

      expect(committedIdentity.toObject()).to.not.deep.equals(identity.toObject());
    });
  });
});
