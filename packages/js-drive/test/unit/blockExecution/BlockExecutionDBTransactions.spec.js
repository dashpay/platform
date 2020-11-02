const BlockExecutionDBTransactions = require('../../../lib/blockExecution/BlockExecutionDBTransactions');

describe('BlockExecutionDBTransactions', () => {
  let blockExecutionDBTransactions;
  let identityTransactionMock;
  let documentsTransactionMock;
  let dataContractsTransactionMock;

  beforeEach(function beforeEach() {
    identityTransactionMock = {
      commit: this.sinon.stub(),
      start: this.sinon.stub(),
      abort: this.sinon.stub(),
    };

    documentsTransactionMock = {
      commit: this.sinon.stub(),
      start: this.sinon.stub(),
      abort: this.sinon.stub(),
    };

    dataContractsTransactionMock = {
      commit: this.sinon.stub(),
      start: this.sinon.stub(),
      abort: this.sinon.stub(),
    };

    blockExecutionDBTransactions = new BlockExecutionDBTransactions(
      identityTransactionMock,
      documentsTransactionMock,
      dataContractsTransactionMock,
    );
  });

  it('should start transactions', () => {
    blockExecutionDBTransactions.start();

    expect(identityTransactionMock.start).to.be.calledOnce();
    expect(documentsTransactionMock.start).to.be.calledOnce();
    expect(dataContractsTransactionMock.start).to.be.calledOnce();

    expect(identityTransactionMock.commit).to.be.not.called();
    expect(documentsTransactionMock.commit).to.be.not.called();
    expect(dataContractsTransactionMock.commit).to.be.not.called();

    expect(identityTransactionMock.abort).to.be.not.called();
    expect(documentsTransactionMock.abort).to.be.not.called();
    expect(dataContractsTransactionMock.abort).to.be.not.called();
  });

  it('should commit transactions', async () => {
    await blockExecutionDBTransactions.commit();

    expect(identityTransactionMock.commit).to.be.calledOnce();
    expect(documentsTransactionMock.commit).to.be.calledOnce();
    expect(dataContractsTransactionMock.commit).to.be.calledOnce();

    expect(identityTransactionMock.start).to.be.not.called();
    expect(documentsTransactionMock.start).to.be.not.called();
    expect(dataContractsTransactionMock.start).to.be.not.called();

    expect(identityTransactionMock.abort).to.be.not.called();
    expect(documentsTransactionMock.abort).to.be.not.called();
    expect(dataContractsTransactionMock.abort).to.be.not.called();
  });

  it('should abort transactions', async () => {
    await blockExecutionDBTransactions.abort();

    expect(identityTransactionMock.abort).to.be.calledOnce();
    expect(documentsTransactionMock.abort).to.be.calledOnce();
    expect(dataContractsTransactionMock.abort).to.be.calledOnce();

    expect(identityTransactionMock.start).to.be.not.called();
    expect(documentsTransactionMock.start).to.be.not.called();
    expect(dataContractsTransactionMock.start).to.be.not.called();

    expect(identityTransactionMock.commit).to.be.not.called();
    expect(documentsTransactionMock.commit).to.be.not.called();
    expect(dataContractsTransactionMock.commit).to.be.not.called();
  });

  it('should return transaction by name', () => {
    const result = blockExecutionDBTransactions.getTransaction('identity');

    expect(result).to.deep.equal(identityTransactionMock);
  });
});
