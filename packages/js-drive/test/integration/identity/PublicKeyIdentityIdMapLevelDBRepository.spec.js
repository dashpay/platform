const level = require('level-rocksdb');

const IdentityPublicKey = require('@dashevo/dpp/lib/identity/IdentityPublicKey');
const getIdentityFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityFixture');

const LevelDBTransaction = require('../../../lib/levelDb/LevelDBTransaction');

const PublicKeyIdentityIdMapLevelDBRepository = require(
  '../../../lib/identity/PublicKeyIdentityIdMapLevelDBRepository',
);

describe('PublicKeyIdentityIdMapLevelDBRepository', () => {
  let db;
  let repository;
  let identity;
  let publicKey;
  let key;

  beforeEach(() => {
    db = level('./db/identity-test', { valueEncoding: 'binary' });

    identity = getIdentityFixture();
    publicKey = new IdentityPublicKey({
      id: 0,
      type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
      data: '02c18a9df635941906c9b4cb8b49fe1070b7469c38d4a5c8723049104d8e008976',
      isEnabled: true,
    });

    repository = new PublicKeyIdentityIdMapLevelDBRepository(db);

    key = `${PublicKeyIdentityIdMapLevelDBRepository.KEY_PREFIX}:${publicKey.hash()}`;
  });

  afterEach(async () => {
    await db.clear();
    await db.close();
  });

  describe('#store', () => {
    it('should store identity id', async () => {
      const repositoryInstance = await repository.store(publicKey.hash(), identity.getId());

      expect(repositoryInstance).to.equal(repository);

      const storedIdentityId = await db.get(key);

      expect(storedIdentityId).to.be.instanceOf(Buffer);
      expect(storedIdentityId.toString()).to.deep.equal(identity.getId());
    });

    it('should store identity id in transaction', async () => {
      const transaction = repository.createTransaction();

      expect(transaction).to.be.instanceOf(LevelDBTransaction);

      transaction.start();
      // store data in transaction
      await repository.store(publicKey.hash(), identity.getId(), transaction);

      // check we don't have data in db before commit
      try {
        await db.get(key);

        expect.fail('Should fail with NotFoundError error');
      } catch (e) {
        expect(e.type).to.equal('NotFoundError');
      }

      // check we can't fetch data without transaction
      const notFoundIdentity = await repository.fetch(publicKey.hash());

      expect(notFoundIdentity).to.be.null();

      // check we can fetch data inside transaction
      const identityIdFromTransaction = await repository.fetch(publicKey.hash(), transaction);

      expect(identityIdFromTransaction).to.deep.equal(identity.getId());

      await transaction.commit();

      // check we have data in db after commit
      const storedIdentityIdBuffer = await db.get(key);

      expect(storedIdentityIdBuffer).to.be.instanceOf(Buffer);
      expect(storedIdentityIdBuffer.toString()).to.deep.equal(identity.getId());
    });
  });

  describe('#fetch', () => {
    it('should return null if identity id was not found', async () => {
      await repository.store(publicKey.hash(), identity.getId());

      const storedIdentityId = await repository.fetch('nonExistingId');

      expect(storedIdentityId).to.be.null();
    });

    it('should return stored identity id', async () => {
      await db.put(key, identity.getId());

      const storedIdentityId = await repository.fetch(publicKey.hash());

      expect(storedIdentityId).to.deep.equal(identity.getId());
    });

    it('should return stored identity id with transaction', async () => {
      await repository.store(publicKey.hash(), identity.getId());

      const transaction = repository.createTransaction();

      transaction.start();
      const storedIdentityId = await repository.fetch(publicKey.hash(), transaction);

      expect(storedIdentityId).to.deep.equal(identity.getId());
    });

    it('should return null if identity not found', async () => {
      const storedIdentityId = await repository.fetch(null);

      expect(storedIdentityId).to.equal(null);
    });
  });
});
