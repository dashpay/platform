const fs = require('fs');
const { PrivateKey } = require('@dashevo/dashcore-lib');
const Drive = require('@dashevo/rs-drive');
const getIdentityFixture = require('@dashevo/wasm-dpp/lib/test/fixtures/getIdentityFixture');

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
  let IdentityPublicKey;
  let decodeProtocolEntity;

  before(function before() {
    ({ IdentityPublicKey, decodeProtocolEntity } = this.dppWasm);
  });

  beforeEach(async function beforeEach() {
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
    }, this.dppWasm);

    store = new GroveDBStore(rsDrive, logger);

    await rsDrive.createInitialStateStructure();

    identityRepository = new IdentityStoreRepository(store, decodeProtocolEntity, this.dppWasm);

    publicKeyRepository = new IdentityPublicKeyStoreRepository(
      store, decodeProtocolEntity, this.dppWasm,
    );

    identity = await getIdentityFixture();

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

      const privateKey = new PrivateKey();
      const publicKeyData = privateKey.toPublicKey().toBuffer();

      const pk = {
        ...publicKey.toObject(),
        id: 12,
        data: publicKeyData,
      };

      const result = await publicKeyRepository.add(
        identity.getId(),
        [
          pk,
        ],
        blockInfo,
      );

      expect(result).to.be.instanceOf(StorageResult);
      expect(result.getOperations().length).to.equal(1);

      const fetchedIdentityResult = await identityRepository.fetch(identity.getId());

      const fetchedIdentity = fetchedIdentityResult.getValue();

      const newPublicKeys = [
        ...identity.getPublicKeys(),
        new IdentityPublicKey(pk),
      ];

      identity.setPublicKeys(newPublicKeys);

      expect(fetchedIdentity.toObject()).to.deep.equals(identity.toObject());
    });

    it('should store public key to identities using transaction', async () => {
      const publicKey = identity.getPublicKeys().pop();

      await identityRepository.create(identity, blockInfo);

      await store.startTransaction();

      const privateKey = new PrivateKey();
      const publicKeyData = privateKey.toPublicKey().toBuffer();

      const pk = {
        ...publicKey.toObject(),
        id: 12,
        data: publicKeyData,
      };

      await publicKeyRepository.add(
        identity.getId(),
        [
          pk,
        ],
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

      const newPublicKeys = [
        ...identity.getPublicKeys(),
        new IdentityPublicKey(pk),
      ];

      identity.setPublicKeys(newPublicKeys);

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

      const disabledAt = Date.now();

      const result = await publicKeyRepository.disable(
        identity.getId(),
        [0, 1],
        disabledAt,
        blockInfo,
      );

      expect(result).to.be.instanceOf(StorageResult);
      expect(result.getOperations().length).to.equal(1);

      const fetchedIdentityResult = await identityRepository.fetch(identity.getId());

      const fetchedIdentity = fetchedIdentityResult.getValue();

      fetchedIdentity.getPublicKeys().forEach((pk) => {
        expect(pk.toObject().disabledAt).to.equal(disabledAt);
      });
    });

    it('should store public key to identities using transaction', async () => {
      await identityRepository.create(identity, blockInfo);

      await store.startTransaction();

      const disabledAt = Date.now();

      await publicKeyRepository.disable(
        identity.getId(),
        [0, 1],
        disabledAt,
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

      transactionalIdentity.getPublicKeys().forEach((pk) => {
        expect(pk.toObject().disabledAt).to.equal(disabledAt);
      });

      await store.commitTransaction();

      const committedIdentityResult = await identityRepository.fetch(identity.getId());

      const committedIdentity = committedIdentityResult.getValue();

      committedIdentity.getPublicKeys().forEach((pk) => {
        expect(pk.toObject().disabledAt).to.equal(disabledAt);
      });
    });
  });
});
