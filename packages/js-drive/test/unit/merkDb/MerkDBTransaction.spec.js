const merk = require('merk');
const rimraf = require('rimraf');

const MerkDbTransaction = require('../../../lib/merkDb/MerkDbTransaction');
const MerkDbTransactionWrapper = require('../../../lib/merkDb/MerkDbInMemoryDecorator');

const MerkDBTransactionIsNotStartedError = require('../../../lib/merkDb/errors/MerkDBTransactionIsNotStartedError');
const MerkDBTransactionIsAlreadyStartedError = require('../../../lib/merkDb/errors/MerkDBTransactionIsAlreadyStartedError');

describe('MerkDbTransaction', () => {
  let dbMock;
  let merkDBTransaction;
  let dbPath;

  beforeEach(() => {
    dbPath = './db/identity-test';
    dbMock = merk(`${dbPath}/${Math.random()}`);
    merkDBTransaction = new MerkDbTransaction(dbMock);
  });

  after(async () => {
    rimraf.sync(dbPath);
  });

  describe('#start', () => {
    it('should start transaction', () => {
      merkDBTransaction.start();

      expect(merkDBTransaction.db).to.be.instanceOf(MerkDbTransactionWrapper);
    });

    it('should throw LevelDBTransactionIsAlreadyStartedError if transaction was started twice', async () => {
      merkDBTransaction.start();

      try {
        merkDBTransaction.start();

        expect.fail('Should throw an LevelDBTransactionIsAlreadyStartedError error');
      } catch (e) {
        expect(e).to.be.instanceOf(MerkDBTransactionIsAlreadyStartedError);
      }
    });
  });

  describe('#commit', () => {
    it('should commit transaction', async function it() {
      const commit = this.sinon.stub();
      merkDBTransaction.db = {
        commit,
      };

      const result = await merkDBTransaction.commit();

      expect(result).to.be.instanceOf(Object);
      expect(commit).to.be.calledOnce();
    });

    it('should throw LevelDBTransactionIsNotStartedError if transaction is not started', async () => {
      try {
        await merkDBTransaction.commit();

        expect.fail('Should throw an LevelDBTransactionIsNotStartedError error');
      } catch (e) {
        expect(e).to.be.instanceOf(MerkDBTransactionIsNotStartedError);
      }
    });
  });

  describe('#abort', () => {
    it('should abort transaction', async function it() {
      const rollback = this.sinon.stub();
      merkDBTransaction.db = {
        rollback,
      };

      const result = await merkDBTransaction.abort();

      expect(result).to.be.instanceOf(Object);
      expect(rollback).to.be.calledOnce();
    });

    it('should throw LevelDBTransactionIsAlreadyStartedError if transaction is not started', async () => {
      try {
        await merkDBTransaction.abort();

        expect.fail('should throw LevelDBTransactionIsAlreadyStartedError');
      } catch (e) {
        expect(e).to.be.an.instanceof(MerkDBTransactionIsNotStartedError);
      }
    });
  });

  describe('#isStarted', () => {
    it('should return true if transaction is started', async () => {
      expect(merkDBTransaction.isStarted()).to.be.false();

      await merkDBTransaction.start();

      expect(merkDBTransaction.isStarted()).to.be.true();
    });

    it('should return false if transaction is aborted', async () => {
      expect(merkDBTransaction.isStarted()).to.be.false();

      await merkDBTransaction.start();

      expect(merkDBTransaction.isStarted()).to.be.true();

      await merkDBTransaction.abort();

      expect(merkDBTransaction.isStarted()).to.be.false();
    });
  });
});
