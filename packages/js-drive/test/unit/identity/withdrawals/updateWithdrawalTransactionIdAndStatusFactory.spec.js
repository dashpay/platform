const getDocumentFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentsFixture');
const Identifier = require('@dashevo/dpp/lib/identifier/Identifier');

const updateWithdrawalTransactionIdAndStatusFactory = require('../../../../lib/identity/withdrawals/updateWithdrawalTransactionIdAndStatusFactory');
const BlockInfo = require('../../../../lib/blockExecution/BlockInfo');

describe('updateWithdrawalTransactionIdAndStatusFactory', () => {
  let updateWithdrawalTransactionIdAndStatus;
  let withdrawalsContractId;
  let documentRepositoryMock;
  let fetchDocumentsMock;
  let documentFixture;

  beforeEach(function beforeEach() {
    documentFixture = getDocumentFixture();

    withdrawalsContractId = Identifier.from(Buffer.alloc(32));

    documentRepositoryMock = {
      update: this.sinon.stub(),
    };

    fetchDocumentsMock = this.sinon.stub();
    fetchDocumentsMock.resolves([documentFixture[0]]);

    updateWithdrawalTransactionIdAndStatus = updateWithdrawalTransactionIdAndStatusFactory(
      documentRepositoryMock,
      fetchDocumentsMock,
      withdrawalsContractId,
    );
  });

  it('should update documents transactionId, status and revision', async () => {
    const blockInfo = new BlockInfo(1, 1, 1);

    const updatedTxId = Buffer.alloc(32, 2);

    await updateWithdrawalTransactionIdAndStatus(blockInfo, Buffer.alloc(0), updatedTxId, {
      useTransaction: true,
    });

    expect(fetchDocumentsMock).to.have.been.calledOnceWithExactly(
      withdrawalsContractId,
      'withdrawals',
      {
        where: [
          ['status', '==', 1],
          ['transactionId', '==', Buffer.alloc(0)],
        ],
        useTransaction: true,
      },
    );

    expect(documentRepositoryMock.update).to.have.been.calledOnceWithExactly(
      documentFixture[0], blockInfo, { useTransaction: true },
    );

    expect(documentFixture[0].get('transactionId')).to.deep.equal(updatedTxId);
    expect(documentFixture[0].get('status')).to.deep.equal(2);
    expect(documentFixture[0].getRevision()).to.deep.equal(2);
  });
});
