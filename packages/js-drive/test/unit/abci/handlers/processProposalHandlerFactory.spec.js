const {
  tendermint: {
    abci: {
      ResponseProcessProposal,
      ValidatorSetUpdate,
    },
    types: {
      ConsensusParams,
    },
  },
} = require('@dashevo/abci/types');
const Long = require('long');

const processProposalHandlerFactory = require('../../../../lib/abci/handlers/processProposalHandlerFactory');
const LoggerMock = require('../../../../lib/test/mock/LoggerMock');
const BlockExecutionContextMock = require('../../../../lib/test/mock/BlockExecutionContextMock');

describe('processProposalHandlerFactory', () => {
  let processProposalHandler;
  let request;
  let blockExecutionContextMock;
  let loggerMock;
  let beginBlockMock;
  let endBlockMock;
  let verifyChainLockMock;
  let deliverTxMock;
  let appHash;
  let validatorSetUpdate;
  let consensusParamUpdates;
  let coreChainLockUpdate;
  let proposalBlockExecutionContextCollectionMock;
  let round;

  beforeEach(function beforeEach() {
    round = 0;
    appHash = Buffer.alloc(1, 1);

    blockExecutionContextMock = new BlockExecutionContextMock(this.sinon);

    loggerMock = new LoggerMock(this.sinon);

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

    beginBlockMock = this.sinon.stub();
    endBlockMock = this.sinon.stub().resolves({
      consensusParamUpdates,
      appHash,
      validatorSetUpdate,
    });
    deliverTxMock = this.sinon.stub().resolves({
      code: 0,
      processingFees: 1,
      storageFees: 2,
    });
    beginBlockMock = this.sinon.stub();
    verifyChainLockMock = this.sinon.stub().resolves(true);

    proposalBlockExecutionContextCollectionMock = {
      get: this.sinon.stub().returns(blockExecutionContextMock),
    };

    processProposalHandler = processProposalHandlerFactory(
      deliverTxMock,
      loggerMock,
      proposalBlockExecutionContextCollectionMock,
      beginBlockMock,
      endBlockMock,
      verifyChainLockMock,
    );

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
    const proposedLastCommit = {};

    coreChainLockUpdate = {
      coreBlockHeight: 42,
      coreBlockHash: '1528e523f4c20fa84ba70dd96372d34e00ce260f357d53ad1a8bc892ebf20e2d',
      signature: '1897ce8f54d2070f44ca5c29983b68b391e8137c25e44f67416e579f3e3bdfef7b4fd22db7818399147e52907998857b0fbc8edfdc40a64f2c7df0e88544d31d12ca8c15e73d50dda25ca23f754ed3f789ed4bcb392161995f464017c10df404',
    };

    request = {
      round,
      height,
      txs,
      coreChainLockedHeight,
      version,
      proposedLastCommit,
      time,
      proposerProTxHash,
      coreChainLockUpdate,
    };
  });

  it('should return ResponseProcessProposal', async () => {
    const result = await processProposalHandler(request);

    expect(result).to.be.an.instanceOf(ResponseProcessProposal);
    expect(result.status).to.equal(1);
    expect(result.appHash).to.equal(appHash);
    expect(result.txResults).to.be.deep.equal(new Array(3).fill({ code: 0 }));
    expect(result.consensusParamUpdates).to.be.equal(consensusParamUpdates);
    expect(result.validatorSetUpdate).to.be.equal(validatorSetUpdate);

    expect(proposalBlockExecutionContextCollectionMock.get).to.be.calledOnceWithExactly(round);

    expect(beginBlockMock).to.be.calledOnceWithExactly(
      {
        lastCommitInfo: request.proposedLastCommit,
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

    expect(verifyChainLockMock).to.be.calledOnceWithExactly(
      coreChainLockUpdate,
    );

    expect(endBlockMock).to.be.calledOnceWithExactly({
      height: request.height,
      round,
      processingFees: 3,
      storageFees: 6,
      coreChainLockedHeight: request.coreChainLockedHeight,
    },
    loggerMock);
  });

  it('should return rejected ResponseProcessProposal if chainlock can\'t be verified', async () => {
    verifyChainLockMock.resolves(false);

    const result = await processProposalHandler(request);

    expect(result).to.be.an.instanceOf(ResponseProcessProposal);
    expect(result.status).to.equal(2);
  });
});
