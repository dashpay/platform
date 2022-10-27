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
  let blockExecutionContextMock;
  let latestBlockExecutionContextMock;
  let loggerMock;
  let requestMock;
  let appHash;
  let groveDBStoreMock;
  let coreRpcClientMock;
  let dataContractCacheMock;
  let blockExecutionContextRepositoryMock;
  let dataContract;
  let proposalBlockExecutionContextCollectionMock;
  let round;

  beforeEach(function beforeEach() {
    round = 0;
    appHash = Buffer.alloc(0);
    blockExecutionContextMock = new BlockExecutionContextMock(this.sinon);
    latestBlockExecutionContextMock = new BlockExecutionContextMock(this.sinon);
    loggerMock = new LoggerMock(this.sinon);
    executionTimerMock = {
      clearTimer: this.sinon.stub(),
      startTimer: this.sinon.stub(),
      stopTimer: this.sinon.stub(),
    };

    const decidedLastCommit = {};

    const height = new Long(42);

    const time = {
      seconds: Math.ceil(new Date().getTime() / 1000),
    };

    const coreChainLockedHeight = 10;

    requestMock = {
      decidedLastCommit,
      height,
      time,
      coreChainLockedHeight,
      round,
    };

    dataContract = getDataContractFixture();

    blockExecutionContextMock.getHeight.returns(42);
    blockExecutionContextMock.getDataContracts.returns([dataContract]);

    dataContractCacheMock = {
      set: this.sinon.stub(),
      get: this.sinon.stub(),
      has: this.sinon.stub(),
    };

    groveDBStoreMock = new GroveDBStoreMock(this.sinon);
    groveDBStoreMock.getRootHash.resolves(appHash);

    coreRpcClientMock = {
      sendRawTransaction: this.sinon.stub(),
    };

    blockExecutionContextRepositoryMock = new BlockExecutionContextRepositoryMock(
      this.sinon,
    );

    proposalBlockExecutionContextCollectionMock = {
      get: this.sinon.stub().returns(blockExecutionContextMock),
      clear: this.sinon.stub(),
    };

    finalizeBlockHandler = finalizeBlockHandlerFactory(
      groveDBStoreMock,
      blockExecutionContextRepositoryMock,
      proposalBlockExecutionContextCollectionMock,
      dataContractCacheMock,
      coreRpcClientMock,
      loggerMock,
      executionTimerMock,
      latestBlockExecutionContextMock,
    );
  });

  it('should commit db transactions, create document dbs and return ResponseFinalizeBlock', async () => {
    const result = await finalizeBlockHandler(requestMock);

    expect(result).to.be.an.instanceOf(ResponseFinalizeBlock);

    expect(executionTimerMock.stopTimer).to.be.calledOnceWithExactly('blockExecution');

    expect(proposalBlockExecutionContextCollectionMock.get).to.be.calledOnceWithExactly(round);
    expect(proposalBlockExecutionContextCollectionMock.clear).to.be.calledOnce();

    expect(blockExecutionContextRepositoryMock.store).to.be.calledOnceWithExactly(
      blockExecutionContextMock,
      {
        useTransaction: true,
      },
    );

    expect(groveDBStoreMock.commitTransaction).to.be.calledOnceWithExactly();

    expect(blockExecutionContextMock.getDataContracts).to.be.calledOnce();
    expect(latestBlockExecutionContextMock.populate).to.be.calledOnce();
  });

  it('should send withdrawal transaction if vote extensions are present', async () => {
    const [txOneBytes, txTwoBytes] = [
      Buffer.alloc(32, 0),
      Buffer.alloc(32, 1),
    ];

    blockExecutionContextMock.getWithdrawalTransactionsMap.returns({
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

    requestMock.decidedLastCommit = { thresholdVoteExtensions };

    await finalizeBlockHandler(requestMock);

    expect(coreRpcClientMock.sendRawTransaction).to.have.been.calledTwice();
  });
});
