const MerkDbTransaction = require('../../../lib/merkDb/MerkDbTransaction');
const MerkDbTransactionWrapper = require('../../../lib/merkDb/MerkDbInMemoryDecorator');

const MerkDBTransactionIsNotStartedError = require('../../../lib/merkDb/errors/MerkDBTransactionIsNotStartedError');
const MerkDBTransactionIsAlreadyStartedError = require('../../../lib/merkDb/errors/MerkDBTransactionIsAlreadyStartedError');

describe('MerkDbTransaction', () => {
  let merkDBTransaction;

  beforeEach(() => {
    const merkDbMock = {};

    merkDBTransaction = new MerkDbTransaction(merkDbMock);
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
      const persist = this.sinon.stub();

      merkDBTransaction.db = {
        persist,
      };

      const result = await merkDBTransaction.commit();

      expect(result).to.be.instanceOf(Object);
      expect(persist).to.be.calledOnce();
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
      const reset = this.sinon.stub();

      merkDBTransaction.db = {
        reset,
      };

      const result = await merkDBTransaction.abort();

      expect(result).to.be.instanceOf(Object);
      expect(reset).to.be.calledOnce();
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
