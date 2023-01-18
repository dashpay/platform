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
  let loggerMock;
  let verifyChainLockMock;
  let coreChainLockUpdate;
  let processProposalMock;
  let round;
  let appHash;
  let proposalBlockExecutionContextMock;
  let consensusParamUpdates;
  let validatorSetUpdate;
  let createContextLoggerMock;

  beforeEach(function beforeEach() {
    round = 0;

    appHash = Buffer.alloc(1, 1);

    proposalBlockExecutionContextMock = new BlockExecutionContextMock(this.sinon);

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

    loggerMock = new LoggerMock(this.sinon);

    verifyChainLockMock = this.sinon.stub().resolves(true);

    processProposalMock = this.sinon.stub().resolves(new ResponseProcessProposal({ status: 1 }));
    createContextLoggerMock = this.sinon.stub().returns(loggerMock);

    processProposalHandler = processProposalHandlerFactory(
      loggerMock,
      verifyChainLockMock,
      processProposalMock,
      proposalBlockExecutionContextMock,
      createContextLoggerMock,
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

    expect(processProposalMock).to.be.calledOnceWithExactly(request, loggerMock);

    expect(verifyChainLockMock).to.be.calledOnceWithExactly(
      coreChainLockUpdate,
    );
  });

  it('should return rejected ResponseProcessProposal if chainlock can\'t be verified', async () => {
    verifyChainLockMock.resolves(false);

    const result = await processProposalHandler(request);

    expect(result).to.be.an.instanceOf(ResponseProcessProposal);
    expect(result.status).to.equal(2);

    expect(processProposalMock).to.not.be.called();
  });

  it('should return already prepared result for this height and round', async () => {
    proposalBlockExecutionContextMock.getHeight.returns(request.height);
    proposalBlockExecutionContextMock.getRound.returns(request.round);

    proposalBlockExecutionContextMock.getPrepareProposalResult.returns({
      appHash,
      txResults: new Array(3).fill({ code: 0 }),
      consensusParamUpdates,
      validatorSetUpdate,
    });

    const result = await processProposalHandler(request);

    expect(proposalBlockExecutionContextMock.getPrepareProposalResult).to.be.calledOnce();

    expect(result).to.be.an.instanceOf(ResponseProcessProposal);
    expect(result.status).to.equal(1);
    expect(result.appHash).to.equal(appHash);
    expect(result.txResults).to.be.deep.equal(new Array(3).fill({ code: 0 }));
    expect(result.consensusParamUpdates).to.be.equal(consensusParamUpdates);
    expect(result.validatorSetUpdate).to.be.equal(validatorSetUpdate);

    expect(processProposalMock).to.not.be.called();

    expect(verifyChainLockMock).to.not.be.called();
    expect(createContextLoggerMock).to.be.calledOnceWithExactly(loggerMock, {
      height: '42',
      round,
      abciMethod: 'processProposal',
    });
  });
});
