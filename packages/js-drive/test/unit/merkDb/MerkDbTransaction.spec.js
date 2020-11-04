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
    it('should start transaction', async () => {
      await merkDBTransaction.start();

      expect(merkDBTransaction.db).to.be.instanceOf(MerkDbTransactionWrapper);
    });

    it('should throw MerkDBTransactionIsAlreadyStartedError if transaction was started twice', async () => {
      await merkDBTransaction.start();

      try {
        await merkDBTransaction.start();

        expect.fail('Should throw an MerkDBTransactionIsAlreadyStartedError error');
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

    it('should throw MerkDBTransactionIsNotStartedError if transaction is not started', async () => {
      try {
        await merkDBTransaction.commit();

        expect.fail('Should throw an MerkDBTransactionIsNotStartedError error');
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

    it('should throw MerkDBTransactionIsNotStartedError if transaction is not started', async () => {
      try {
        await merkDBTransaction.abort();

        expect.fail('should throw MerkDBTransactionIsNotStartedError');
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
