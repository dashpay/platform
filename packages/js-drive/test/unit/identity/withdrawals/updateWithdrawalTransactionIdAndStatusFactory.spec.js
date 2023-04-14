const getDocumentsFixture = require('@dashevo/wasm-dpp/lib/test/fixtures/getDocumentsFixture');

const updateWithdrawalTransactionIdAndStatusFactory = require('../../../../lib/identity/withdrawals/updateWithdrawalTransactionIdAndStatusFactory');
const BlockInfo = require('../../../../lib/blockExecution/BlockInfo');

describe('updateWithdrawalTransactionIdAndStatusFactory', () => {
  let updateWithdrawalTransactionIdAndStatus;
  let withdrawalsContractId;
  let documentRepositoryMock;
  let fetchDocumentsMock;
  let document1Fixture;
  let document2Fixture;
  let Identifier;

  before(function before() {
    ({ Identifier } = this.dppWasm);
  });

  beforeEach(async function beforeEach() {
    ([document1Fixture, document2Fixture] = await getDocumentsFixture());

    document1Fixture.set('transactionId', Buffer.alloc(32, 1));
    document2Fixture.set('transactionId', Buffer.alloc(32, 3));

    withdrawalsContractId = Identifier.from(Buffer.alloc(32));

    documentRepositoryMock = {
      update: this.sinon.stub(),
    };

    fetchDocumentsMock = this.sinon.stub();
    fetchDocumentsMock.resolves([document1Fixture, document2Fixture]);

    updateWithdrawalTransactionIdAndStatus = updateWithdrawalTransactionIdAndStatusFactory(
      documentRepositoryMock,
      fetchDocumentsMock,
      withdrawalsContractId,
    );
  });

  it('should update documents transactionId, status and revision', async () => {
    const blockInfo = new BlockInfo(1, 1, 1);

    const coreChainLockedHeight = 42;

    const transactionIdMap = {
      [Buffer.alloc(32, 1).toString('hex')]: Buffer.alloc(32, 2),
      [Buffer.alloc(32, 3).toString('hex')]: Buffer.alloc(32, 4),
    };

    await updateWithdrawalTransactionIdAndStatus(
      blockInfo,
      coreChainLockedHeight,
      transactionIdMap,
      {
        useTransaction: true,
      },
    );

    expect(fetchDocumentsMock).to.have.been.calledOnceWithExactly(
      withdrawalsContractId,
      'withdrawal',
      {
        where: [
          ['status', '==', 1],
          ['transactionId', 'in', [Buffer.alloc(32, 1), Buffer.alloc(32, 3)]],
        ],
        orderBy: [
          ['transactionId', 'asc'],
        ],
        useTransaction: true,
      },
    );

    expect(documentRepositoryMock.update).to.have.been.calledTwice();
    expect(documentRepositoryMock.update.getCall(0).args).to.deep.equal(
      [document1Fixture, blockInfo, { useTransaction: true }],
    );
    expect(documentRepositoryMock.update.getCall(1).args).to.deep.equal(
      [document2Fixture, blockInfo, { useTransaction: true }],
    );

    expect(document1Fixture.get('transactionSignHeight')).to.deep.equal(coreChainLockedHeight);
    expect(document1Fixture.get('transactionId')).to.deep.equal(Buffer.alloc(32, 2));
    expect(document1Fixture.get('status')).to.deep.equal(2);
    expect(document1Fixture.getRevision()).to.deep.equal(2);

    expect(document2Fixture.get('transactionSignHeight')).to.deep.equal(coreChainLockedHeight);
    expect(document2Fixture.get('transactionId')).to.deep.equal(Buffer.alloc(32, 4));
    expect(document2Fixture.get('status')).to.deep.equal(2);
    expect(document2Fixture.getRevision()).to.deep.equal(2);
  });
});
