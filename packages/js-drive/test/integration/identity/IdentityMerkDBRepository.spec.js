const rimraf = require('rimraf');
const merk = require('merk');
const cbor = require('cbor');

const getIdentityFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityFixture');
const createDPPMock = require('@dashevo/dpp/lib/test/mocks/createDPPMock');
const Identity = require('@dashevo/dpp/lib/identity/Identity');
const generateRandomIdentifier = require('@dashevo/dpp/lib/test/utils/generateRandomIdentifier');

const MerkDbTransaction = require('../../../lib/merkDb/MerkDbTransaction');

const IdentityMerkDBRepository = require('../../../lib/identity/IdentityMerkDBRepository');

describe('IdentityMerkDBRepository', () => {
  let dbPath;
  let db;
  let repository;
  let identity;
  let dppMock;

  beforeEach(function beforeEach() {
    dbPath = './db/identity-test';
    db = merk(`${dbPath}/${Math.random()}`);

    identity = getIdentityFixture();

    dppMock = createDPPMock(this.sinon);
    dppMock
      .identity
      .createFromBuffer
      .resolves(identity);

    repository = new IdentityMerkDBRepository(db, dppMock);
  });

  after(async () => {
    rimraf.sync(dbPath);
  });

  describe('#store', () => {
    it('should store identity', async () => {
      const repositoryInstance = await repository.store(identity);

      expect(repositoryInstance).to.equal(repository);

      const storedIdentityBuffer = db.getSync(identity.getId());

      expect(storedIdentityBuffer).to.be.instanceOf(Buffer);

      const storedIdentity = cbor.decode(storedIdentityBuffer);

      expect(storedIdentity).to.deep.equal(identity.toObject());
    });

    it('should store identity in transaction', async () => {
      const transaction = repository.createTransaction();

      expect(transaction).to.be.instanceOf(MerkDbTransaction);

      transaction.start();
      // store data in transaction
      await repository.store(identity, transaction);

      // check we don't have data in db before commit
      try {
        db.getSync(identity.getId());

        expect.fail('Should fail with NotFoundError error');
      } catch (e) {
        expect(e.message.startsWith('key not found')).to.be.true();
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
      const storedIdentityBuffer = db.getSync(identity.getId());

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
      db.batch().put(identity.getId(), identity.toBuffer()).commitSync();

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
