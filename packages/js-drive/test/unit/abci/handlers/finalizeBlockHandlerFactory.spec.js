const {
  tendermint: {
    abci: {
      ResponseFinalizeBlock,
    },
  },
} = require('@dashevo/abci/types');

const Long = require('long');

const { hash } = require('@dashevo/dpp/lib/util/hash');
const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const finalizeBlockHandlerFactory = require('../../../../lib/abci/handlers/finalizeBlockHandlerFactory');
const LoggerMock = require('../../../../lib/test/mock/LoggerMock');
const BlockExecutionContextMock = require('../../../../lib/test/mock/BlockExecutionContextMock');
const GroveDBStoreMock = require('../../../../lib/test/mock/GroveDBStoreMock');
const BlockExecutionContextRepositoryMock = require('../../../../lib/test/mock/BlockExecutionContextRepositoryMock');

describe('finalizeBlockHandlerFactory', () => {
  let finalizeBlockHandler;
  let executionTimerMock;
  let latestBlockExecutionContextMock;
  let loggerMock;
  let requestMock;
  let appHash;
  let groveDBStoreMock;
  let coreRpcClientMock;
  let blockExecutionContextRepositoryMock;
  let dataContract;
  let proposalBlockExecutionContextMock;
  let round;
  let block;
  let processProposalHandlerMock;

  beforeEach(function beforeEach() {
    round = 0;
    appHash = Buffer.alloc(0);
    proposalBlockExecutionContextMock = new BlockExecutionContextMock(this.sinon);

    latestBlockExecutionContextMock = new BlockExecutionContextMock(this.sinon);
    loggerMock = new LoggerMock(this.sinon);
    executionTimerMock = {
      clearTimer: this.sinon.stub(),
      startTimer: this.sinon.stub(),
      stopTimer: this.sinon.stub(),
    };

    const commit = {};

    const height = new Long(42);

    const time = {
      seconds: Math.ceil(new Date().getTime() / 1000),
    };

    const coreChainLockedHeight = 10;

    block = {
      header: {
        time,
        version: {
          app: Long.fromInt(1),
        },
        proposerProTxHash: Uint8Array.from([1, 2, 3, 4]),
        coreChainLockedHeight,
      },
      data: {
        txs: new Array(3).fill(Buffer.alloc(5, 0)),
      },
    };

    requestMock = {
      commit,
      height,
      time,
      coreChainLockedHeight,
      round,
      block,
    };

    dataContract = getDataContractFixture();

    proposalBlockExecutionContextMock.getHeight.returns(new Long(42));
    proposalBlockExecutionContextMock.getRound.returns(round);
    proposalBlockExecutionContextMock.getDataContracts.returns([dataContract]);

    groveDBStoreMock = new GroveDBStoreMock(this.sinon);
    groveDBStoreMock.getRootHash.resolves(appHash);

    coreRpcClientMock = {
      sendRawTransaction: this.sinon.stub(),
    };

    blockExecutionContextRepositoryMock = new BlockExecutionContextRepositoryMock(
      this.sinon,
    );

    processProposalHandlerMock = this.sinon.stub();

    finalizeBlockHandler = finalizeBlockHandlerFactory(
      groveDBStoreMock,
      blockExecutionContextRepositoryMock,
      coreRpcClientMock,
      loggerMock,
      executionTimerMock,
      latestBlockExecutionContextMock,
      proposalBlockExecutionContextMock,
      processProposalHandlerMock,
    );
  });

  it('should commit db transactions, create document dbs and return ResponseFinalizeBlock', async () => {
    const result = await finalizeBlockHandler(requestMock);

    expect(result).to.be.an.instanceOf(ResponseFinalizeBlock);

    expect(executionTimerMock.stopTimer).to.be.calledOnceWithExactly('blockExecution');

    expect(proposalBlockExecutionContextMock.reset).to.be.calledOnce();

    expect(blockExecutionContextRepositoryMock.store).to.be.calledOnceWithExactly(
      proposalBlockExecutionContextMock,
      {
        useTransaction: true,
      },
    );

    expect(groveDBStoreMock.commitTransaction).to.be.calledOnceWithExactly();

    expect(latestBlockExecutionContextMock.populate).to.be.calledOnce();
    expect(processProposalHandlerMock).to.be.not.called();
  });

  it('should send withdrawal transaction if vote extensions are present', async () => {
    const [txOneBytes, txTwoBytes] = [
      Buffer.alloc(32, 0),
      Buffer.alloc(32, 1),
    ];

    proposalBlockExecutionContextMock.getWithdrawalTransactionsMap.returns({
      [hash(txOneBytes).toString('hex')]: txOneBytes,
      [hash(txTwoBytes).toString('hex')]: txTwoBytes,
    });

    const thresholdVoteExtensions = [
      {
        extension: hash(txOneBytes),
        signature: Buffer.alloc(96, 3),
      },
      {
        extension: hash(txTwoBytes),
        signature: Buffer.alloc(96, 4),
      },
    ];

    requestMock.commit = { thresholdVoteExtensions };

    await finalizeBlockHandler(requestMock);

    expect(coreRpcClientMock.sendRawTransaction).to.have.been.calledTwice();
    expect(processProposalHandlerMock).to.be.not.called();
  });

  it('should call processProposalHandler if round is not equal to execution context', async () => {
    proposalBlockExecutionContextMock.getRound.returns(round + 1);

    const result = await finalizeBlockHandler(requestMock);

    expect(result).to.be.an.instanceOf(ResponseFinalizeBlock);
    expect(processProposalHandlerMock).to.be.calledOnceWithExactly({
      height: requestMock.height,
      txs: block.data.txs,
      coreChainLockedHeight: block.header.coreChainLockedHeight,
      version: block.header.version,
      proposedLastCommit: requestMock.commit,
      time: block.header.time,
      proposerProTxHash: block.header.proposerProTxHash,
      round,
    });
  });
});
