const level = require('level-rocksdb');
const cbor = require('cbor');

const getIdentityFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityFixture');
const createDPPMock = require('@dashevo/dpp/lib/test/mocks/createDPPMock');
const Identity = require('@dashevo/dpp/lib/identity/Identity');
const generateRandomIdentifier = require('@dashevo/dpp/lib/test/utils/generateRandomIdentifier');

const LevelDBTransaction = require('../../../lib/levelDb/LevelDBTransaction');

const IdentityLevelDBRepository = require('../../../lib/identity/IdentityLevelDBRepository');

describe('IdentityLevelDBRepository', () => {
  let db;
  let repository;
  let identity;
  let dppMock;

  beforeEach(function beforeEach() {
    db = level('./db/identity-test', { keyEncoding: 'binary', valueEncoding: 'binary' });

    identity = getIdentityFixture();

    dppMock = createDPPMock(this.sinon);
    dppMock
      .identity
      .createFromBuffer
      .resolves(identity);

    repository = new IdentityLevelDBRepository(db, dppMock);
  });

  afterEach(async () => {
    await db.clear();
    await db.close();
  });

  describe('#store', () => {
    it('should store identity', async () => {
      const repositoryInstance = await repository.store(identity);

      expect(repositoryInstance).to.equal(repository);

      const storedIdentityBuffer = await db.get(identity.getId());

      expect(storedIdentityBuffer).to.be.instanceOf(Buffer);

      const storedIdentity = cbor.decode(storedIdentityBuffer);

      expect(storedIdentity).to.deep.equal(identity.toObject());
    });

    it('should store identity in transaction', async () => {
      const transaction = repository.createTransaction();

      expect(transaction).to.be.instanceOf(LevelDBTransaction);

      transaction.start();
      // store data in transaction
      await repository.store(identity, transaction);

      // check we don't have data in db before commit
      try {
        await db.get(identity.getId());

        expect.fail('Should fail with NotFoundError error');
      } catch (e) {
        expect(e.type).to.equal('NotFoundError');
      }

      // check we can't fetch data without transaction
      const notFoundIdentity = await repository.fetch(identity.getId());

      expect(notFoundIdentity).to.be.null();

      // check we can fetch data inside transaction
      const identityFromTransaction = await repository.fetch(identity.getId(), transaction);

      expect(identityFromTransaction).to.be.instanceOf(Identity);
      expect(identityFromTransaction.toObject()).to.deep.equal(identity.toObject());

      await transaction.commit();

      // check we have data in db after commit
      const storedIdentityBuffer = await db.get(identity.getId());

      expect(storedIdentityBuffer).to.be.instanceOf(Buffer);

      const storedIdentity = cbor.decode(storedIdentityBuffer);

      expect(storedIdentity).to.deep.equal(identity.toObject());
    });
  });

  describe('#fetch', () => {
    it('should return null if identity was not found', async () => {
      await repository.store(identity);

      const storedIdentity = await repository.fetch(generateRandomIdentifier());

      expect(storedIdentity).to.be.null();
    });

    it('should return stored identity', async () => {
      await db.put(identity.getId(), identity.toBuffer());

      const storedIdentity = await repository.fetch(identity.getId());

      expect(storedIdentity.toObject()).to.deep.equal(identity.toObject());
    });

    it('should return stored identity with transaction', async () => {
      await repository.store(identity);

      const transaction = repository.createTransaction();

      transaction.start();
      const storedIdentity = await repository.fetch(identity.getId(), transaction);

      expect(storedIdentity.toObject()).to.deep.equal(identity.toObject());
    });

    it('should return null if identity not found', async () => {
      const storedIdentity = await repository.fetch(generateRandomIdentifier());

      expect(storedIdentity).to.equal(null);
    });
  });
});
