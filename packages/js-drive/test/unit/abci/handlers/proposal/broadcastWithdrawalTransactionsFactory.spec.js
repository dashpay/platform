const Long = require('long');

const BlockExecutionContextMock = require('../../../../../lib/test/mock/BlockExecutionContextMock');

const broadcastWithdrawalTransactionsFactory = require('../../../../../lib/abci/handlers/proposal/broadcastWithdrawalTransactionsFactory');
const BlockInfo = require('../../../../../lib/blockExecution/BlockInfo');

describe('broadcastWithdrawalTransactionsFactory', () => {
  let broadcastWithdrawalTransactions;
  let proposalBlockExecutionContextMock;
  let coreRpcMock;
  let updateWithdrawalTransactionIdAndStatusMock;

  beforeEach(function beforeEach() {
    proposalBlockExecutionContextMock = new BlockExecutionContextMock(this.sinon);

    proposalBlockExecutionContextMock.getEpochInfo.returns({
      currentEpochIndex: 1,
    });
    proposalBlockExecutionContextMock.getHeight.returns(new Long(1));
    proposalBlockExecutionContextMock.getTimeMs.returns(1);

    coreRpcMock = {
      sendRawTransaction: this.sinon.stub(),
    };

    updateWithdrawalTransactionIdAndStatusMock = this.sinon.stub();

    broadcastWithdrawalTransactions = broadcastWithdrawalTransactionsFactory(
      coreRpcMock,
      updateWithdrawalTransactionIdAndStatusMock,
    );
  });

  it('should call Core RPC and call document update function', async () => {
    const extension = Buffer.alloc(32, 2);
    const signature = Buffer.alloc(32, 3);

    const txBytes = Buffer.alloc(32, 1);

    const thresholdVoteExtensions = [
      { extension, signature },
    ];
    const unsignedWithdrawalTransactionsMap = {
      [extension.toString('hex')]: txBytes,
    };

    await broadcastWithdrawalTransactions(
      proposalBlockExecutionContextMock,
      thresholdVoteExtensions,
      unsignedWithdrawalTransactionsMap,
    );

    expect(coreRpcMock.sendRawTransaction).to.have.been.calledOnceWithExactly(
      Buffer.concat([txBytes, signature]).toString('hex'),
    );
    expect(updateWithdrawalTransactionIdAndStatusMock).to.have.been.calledOnceWithExactly(
      BlockInfo.createFromBlockExecutionContext(proposalBlockExecutionContextMock),
      txBytes,
      Buffer.concat([txBytes, signature]),
      {
        useTransaction: true,
      },
    );
  });
});
