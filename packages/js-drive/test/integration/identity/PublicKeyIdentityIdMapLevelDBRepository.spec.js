const level = require('level-rocksdb');

const generateRandomIdentifier = require('@dashevo/dpp/lib/test/utils/generateRandomIdentifier');

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

  beforeEach(() => {
    db = level('./db/identity-test', { keyEncoding: 'binary', valueEncoding: 'binary' });

    identity = getIdentityFixture();
    publicKey = new IdentityPublicKey({
      id: 0,
      type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
      data: Buffer.from('A3NCyQmImEGgr7j2kR+rTumHLORpCGYC6XeCqVODZgSm', 'base64'),
    });

    repository = new PublicKeyIdentityIdMapLevelDBRepository(db);
  });

  afterEach(async () => {
    await db.clear();
    await db.close();
  });

  describe('#store', () => {
    it('should store identity id', async () => {
      const repositoryInstance = await repository.store(publicKey.hash(), identity.getId());

      expect(repositoryInstance).to.equal(repository);

      const storedIdentityId = await db.get(publicKey.hash());

      expect(storedIdentityId).to.be.instanceOf(Buffer);
      expect(storedIdentityId).to.deep.equal(identity.getId());
    });

    it('should store identity id in transaction', async () => {
      const transaction = repository.createTransaction();

      expect(transaction).to.be.instanceOf(LevelDBTransaction);

      transaction.start();
      // store data in transaction
      await repository.store(publicKey.hash(), identity.getId(), transaction);

      // check we don't have data in db before commit
      try {
        await db.get(publicKey.hash());

        expect.fail('Should fail with NotFoundError error');
      } catch (e) {
        expect(e.type).to.equal('NotFoundError');
      }

      // check we can't fetch data without transaction
      const notFoundIdentity = await repository.fetch(publicKey.hash());

      expect(notFoundIdentity).to.be.null();

      const result = await transaction.db.get(publicKey.hash());
      expect(result).to.be.instanceOf(Buffer);

      // check we can fetch data inside transaction
      const identityIdFromTransaction = await repository.fetch(publicKey.hash(), transaction);

      expect(identityIdFromTransaction).to.deep.equal(identity.getId());

      await transaction.commit();

      // check we have data in db after commit
      const storedIdentityIdBuffer = await db.get(publicKey.hash());

      expect(storedIdentityIdBuffer).to.be.instanceOf(Buffer);
      expect(storedIdentityIdBuffer).to.deep.equal(identity.getId());
    });
  });

  describe('#fetch', () => {
    it('should return null if identity id was not found', async () => {
      await repository.store(publicKey.hash(), identity.getId());

      const storedIdentityId = await repository.fetch('nonExistingId');

      expect(storedIdentityId).to.be.null();
    });

    it('should return stored identity id', async () => {
      await db.put(publicKey.hash(), identity.getId());

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
      const storedIdentityId = await repository.fetch(generateRandomIdentifier());

      expect(storedIdentityId).to.equal(null);
    });
  });
});
