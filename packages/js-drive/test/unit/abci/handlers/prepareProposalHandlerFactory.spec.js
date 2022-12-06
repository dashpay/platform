const {
  tendermint: {
    abci: {
      ResponsePrepareProposal,
      ValidatorSetUpdate,
    },
    types: {
      ConsensusParams,
      CoreChainLock,
    },
  },
} = require('@dashevo/abci/types');

const Long = require('long');

const prepareProposalHandlerFactory = require('../../../../lib/abci/handlers/prepareProposalHandlerFactory');
const LoggerMock = require('../../../../lib/test/mock/LoggerMock');
const BlockExecutionContextMock = require('../../../../lib/test/mock/BlockExecutionContextMock');

describe('prepareProposalHandlerFactory', () => {
  let prepareProposalHandler;
  let request;
  let deliverTxMock;
  let loggerMock;
  let beginBlockMock;
  let endBlockMock;
  let updateCoreChainLockMock;
  let appHash;
  let consensusParamUpdates;
  let validatorSetUpdate;
  let coreChainLockUpdate;
  let endBlockResult;
  let proposalBlockExecutionContextMock;
  let round;

  beforeEach(function beforeEach() {
    round = 1;
    appHash = Buffer.alloc(1, 1);
    coreChainLockUpdate = new CoreChainLock({
      coreBlockHeight: 42,
      coreBlockHash: '1528e523f4c20fa84ba70dd96372d34e00ce260f357d53ad1a8bc892ebf20e2d',
      signature: '1897ce8f54d2070f44ca5c29983b68b391e8137c25e44f67416e579f3e3bdfef7b4fd22db7818399147e52907998857b0fbc8edfdc40a64f2c7df0e88544d31d12ca8c15e73d50dda25ca23f754ed3f789ed4bcb392161995f464017c10df404',
    });

    consensusParamUpdates = new ConsensusParams({
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
    });
    validatorSetUpdate = new ValidatorSetUpdate();

    proposalBlockExecutionContextMock = new BlockExecutionContextMock(this.sinon);

    loggerMock = new LoggerMock(this.sinon);

    endBlockResult = {
      consensusParamUpdates,
      appHash,
      validatorSetUpdate,
    };

    beginBlockMock = this.sinon.stub();
    deliverTxMock = this.sinon.stub().resolves({
      code: 0,
      processingFees: 1,
      storageFees: 2,
    });
    endBlockMock = this.sinon.stub().resolves(
      endBlockResult,
    );

    updateCoreChainLockMock = this.sinon.stub().resolves(coreChainLockUpdate);

    prepareProposalHandler = prepareProposalHandlerFactory(
      deliverTxMock,
      loggerMock,
      proposalBlockExecutionContextMock,
      beginBlockMock,
      endBlockMock,
      updateCoreChainLockMock,
    );

    const maxTxBytes = 42;
    const txs = new Array(3).fill(Buffer.alloc(5, 0));

    const height = new Long(42);

    const time = {
      seconds: Math.ceil(new Date().getTime() / 1000),
    };

    const version = {
      app: Long.fromInt(1),
    };

    const proposerProTxHash = Uint8Array.from([1, 2, 3, 4]);

    const coreChainLockedHeight = 10;

    const localLastCommit = {};

    request = {
      height,
      maxTxBytes,
      txs,
      coreChainLockedHeight,
      version,
      localLastCommit,
      time,
      proposerProTxHash,
      round,
    };
  });

  it('should return proposal', async () => {
    const result = await prepareProposalHandler(request);

    expect(result).to.be.an.instanceOf(ResponsePrepareProposal);

    const txRecords = request.txs.map((tx) => ({
      tx,
      action: 1,
    }));

    expect(result).to.be.an.instanceOf(ResponsePrepareProposal);
    expect(result.appHash).to.be.equal(appHash);
    expect(result.txResults).to.be.deep.equal(new Array(3).fill({ code: 0 }));
    expect(result.consensusParamUpdates).to.be.equal(consensusParamUpdates);
    expect(result.validatorSetUpdate).to.be.equal(validatorSetUpdate);
    expect(result.coreChainLockUpdate).to.be.equal(coreChainLockUpdate);
    expect(result.txRecords).to.be.deep.equal(txRecords);

    expect(beginBlockMock).to.be.calledOnceWithExactly(
      {
        lastCommitInfo: request.localLastCommit,
        height: request.height,
        coreChainLockedHeight: request.coreChainLockedHeight,
        version: request.version,
        time: request.time,
        proposerProTxHash: Buffer.from(request.proposerProTxHash),
        round,
      },
      loggerMock,
    );

    expect(deliverTxMock).to.be.calledThrice();

    expect(updateCoreChainLockMock).to.be.calledOnceWithExactly(round, loggerMock);

    expect(endBlockMock).to.be.calledOnceWithExactly(
      {
        height: request.height,
        round,
        processingFees: 3,
        storageFees: 6,
        coreChainLockedHeight: request.coreChainLockedHeight,
      },
      loggerMock,
    );

    expect(proposalBlockExecutionContextMock.setPrepareProposalResult).to.be.calledOnceWithExactly({
      appHash,
      txResults: new Array(3).fill({ code: 0 }),
      consensusParamUpdates,
      validatorSetUpdate,
      coreChainLockUpdate,
      txRecords,
    });
  });

  it('should cut txs that are not fit into the size limit', async () => {
    request.maxTxBytes = 9;

    const result = await prepareProposalHandler(request);

    expect(result).to.be.an.instanceOf(ResponsePrepareProposal);
    expect(result.txRecords).to.have.lengthOf(1);
    expect(result.txRecords).to.deep.equal(
      [{
        tx: request.txs[0],
        action: 1,
      }],
    );
  });
});
