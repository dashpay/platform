const {
  tendermint: {
    abci: {
      ResponseFinalizeBlock,
    },
    types: {
      ConsensusParams,
    },
  },
} = require('@dashevo/abci/types');

const Long = require('long');

const finalizeBlockHandlerFactory = require('../../../../lib/abci/handlers/finalizeBlockHandlerFactory');
const LoggerMock = require('../../../../lib/test/mock/LoggerMock');
const BlockExecutionContextMock = require('../../../../lib/test/mock/BlockExecutionContextMock');

describe('finalizeBlockHandlerFactory', () => {
  let finalizeBlockHandler;
  let executionTimerMock;
  let blockExecutionContextMock;
  let beginBlockMock;
  let deliverTxMock;
  let endBlockMock;
  let commitMock;
  let loggerMock;
  let requestMock;
  let appHash;
  let endBlockResult;

  beforeEach(function beforeEach() {
    appHash = Buffer.alloc(0);
    blockExecutionContextMock = new BlockExecutionContextMock(this.sinon);
    loggerMock = new LoggerMock(this.sinon);
    executionTimerMock = {
      clearTimer: this.sinon.stub(),
      startTimer: this.sinon.stub(),
      stopTimer: this.sinon.stub(),
    };

    endBlockResult = {
      consensusParamUpdates: new ConsensusParams({
        block: {
          maxBytes: 1,
          maxGas: 2,
        },
        evidence: {
          maxAgeDuration: null,
          maxAgeNumBlocks: 1,
          maxBytes: 2,
        },
        version: {
          appVersion: 1,
        },
      }),
      nextCoreChainLockUpdate: undefined,
      validatorSetUpdate: undefined,
    };

    const txs = new Array(3).fill(Buffer.alloc(5, 0));

    const decidedLastCommit = {};

    const height = new Long(42);

    const time = {
      seconds: Math.ceil(new Date().getTime() / 1000),
    };

    const coreChainLockedHeight = 10;

    const version = {
      app: Long.fromInt(1),
    };

    const proposerProTxHash = Uint8Array.from([1, 2, 3, 4]);

    requestMock = {
      txs,
      decidedLastCommit,
      height,
      time,
      coreChainLockedHeight,
      version,
      proposerProTxHash,
    };

    beginBlockMock = this.sinon.stub();
    deliverTxMock = this.sinon.stub().resolves({
      code: 0,
    });
    endBlockMock = this.sinon.stub().resolves(endBlockResult);
    commitMock = this.sinon.stub().resolves({ appHash });

    blockExecutionContextMock.getHeight.returns(42);

    finalizeBlockHandler = finalizeBlockHandlerFactory(
      blockExecutionContextMock,
      beginBlockMock,
      deliverTxMock,
      endBlockMock,
      commitMock,
      loggerMock,
      executionTimerMock,
    );
  });

  it('should finalize block', async () => {
    const result = await finalizeBlockHandler(requestMock);

    expect(result).to.be.an.instanceOf(ResponseFinalizeBlock);
    expect(result.txResults).to.be.deep.equal(new Array(3).fill({ code: 0 }));
    expect(result.appHash).to.be.deep.equal(appHash);
    expect(result.consensusParamUpdates).to.be.deep.equal(endBlockResult.consensusParamUpdates);
    expect(result.nextCoreChainLockUpdate).to.be.null();
    expect(result.validatorSetUpdate).to.be.null();

    expect(executionTimerMock.clearTimer).to.be.calledOnceWithExactly('blockExecution');
    expect(executionTimerMock.startTimer).to.be.calledOnceWithExactly('blockExecution');
    expect(executionTimerMock.stopTimer).to.be.calledOnceWithExactly('blockExecution');

    expect(beginBlockMock).to.be.calledOnceWithExactly(
      {
        lastCommitInfo: requestMock.decidedLastCommit,
        height: requestMock.height,
        coreChainLockedHeight: requestMock.coreChainLockedHeight,
        version: requestMock.version,
        time: requestMock.time,
        proposerProTxHash: Buffer.from(requestMock.proposerProTxHash),
      },
      loggerMock,
    );

    expect(deliverTxMock).to.be.calledThrice();
    expect(deliverTxMock.getCall(0)).to.be.calledWithExactly(requestMock.txs[0], loggerMock);
    expect(deliverTxMock.getCall(0)).to.be.calledWithExactly(requestMock.txs[1], loggerMock);
    expect(deliverTxMock.getCall(0)).to.be.calledWithExactly(requestMock.txs[2], loggerMock);

    expect(endBlockMock).to.be.calledOnceWithExactly(requestMock.height, loggerMock);

    expect(commitMock).to.be.calledOnceWithExactly(requestMock.decidedLastCommit, loggerMock);
  });
});
