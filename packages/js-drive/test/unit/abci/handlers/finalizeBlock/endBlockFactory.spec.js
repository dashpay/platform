const {
  tendermint: {
    abci: {
      ValidatorSetUpdate,
    },
    types: {
      ConsensusParams,
      CoreChainLock,
    },
  },
} = require('@dashevo/abci/types');

const Long = require('long');

const endBlockFactory = require('../../../../../lib/abci/handlers/finalizeBlock/endBlockFactory');

const BlockExecutionContextMock = require('../../../../../lib/test/mock/BlockExecutionContextMock');
const LoggerMock = require('../../../../../lib/test/mock/LoggerMock');

describe('endBlockFactory', () => {
  let endBlock;
  let height;
  let lastCommitInfoMock;
  let blockExecutionContextMock;
  let dpnsContractBlockHeight;
  let latestCoreChainLockMock;
  let loggerMock;
  let createValidatorSetUpdateMock;
  let chainLockMock;
  let validatorSetMock;
  let getFeatureFlagForHeightMock;
  let coreChainLockedHeight;
  let version;

  beforeEach(function beforeEach() {
    coreChainLockedHeight = 2;

    version = {
      app: Long.fromInt(1),
    };

    lastCommitInfoMock = {
      stateSignature: Uint8Array.from('003657bb44d74c371d14485117de43313ca5c2848f3622d691c2b1bf3576a64bdc2538efab24854eb82ae7db38482dbd15a1cb3bc98e55173817c9d05c86e47a5d67614a501414aae6dd1565e59422d1d77c41ae9b38de34ecf1e9f778b2a97b'),
    };

    blockExecutionContextMock = new BlockExecutionContextMock(this.sinon);

    blockExecutionContextMock.hasDataContract.returns(true);
    blockExecutionContextMock.getCoreChainLockedHeight.returns(coreChainLockedHeight);
    blockExecutionContextMock.getVersion.returns(version);
    blockExecutionContextMock.getLastCommitInfo.returns(lastCommitInfoMock);

    chainLockMock = {
      height: 1,
      blockHash: Buffer.alloc(0),
      signature: Buffer.alloc(0),
    };

    latestCoreChainLockMock = {
      getChainLock: this.sinon.stub().returns(chainLockMock),
    };

    loggerMock = new LoggerMock(this.sinon);

    dpnsContractBlockHeight = 2;

    validatorSetMock = {
      rotate: this.sinon.stub(),
      getQuorum: this.sinon.stub(),
    };

    createValidatorSetUpdateMock = this.sinon.stub();

    getFeatureFlagForHeightMock = this.sinon.stub().resolves(null);

    endBlock = endBlockFactory(
      blockExecutionContextMock,
      latestCoreChainLockMock,
      validatorSetMock,
      createValidatorSetUpdateMock,
      getFeatureFlagForHeightMock,
    );

    height = Long.fromInt(dpnsContractBlockHeight);
  });

  it('should finalize a block', async () => {
    endBlock = endBlockFactory(
      blockExecutionContextMock,
      latestCoreChainLockMock,
      validatorSetMock,
      createValidatorSetUpdateMock,
      getFeatureFlagForHeightMock,
    );

    const response = await endBlock(height, loggerMock);

    expect(response).to.deep.equal({
      consensusParamUpdates: undefined,
      validatorSetUpdate: undefined,
      nextCoreChainLockUpdate: undefined,
    });

    expect(blockExecutionContextMock.hasDataContract).to.not.have.been.called();
  });

  it('should return nextCoreChainLockUpdate if latestCoreChainLock above header height', async () => {
    chainLockMock.height = 3;

    const response = await endBlock(height, loggerMock);

    expect(latestCoreChainLockMock.getChainLock).to.have.been.calledOnceWithExactly();

    const expectedCoreChainLock = new CoreChainLock({
      coreBlockHeight: chainLockMock.height,
      coreBlockHash: chainLockMock.blockHash,
      signature: chainLockMock.signature,
    });

    expect(response.nextCoreChainLockUpdate).to.deep.equal(expectedCoreChainLock);
    expect(response.validatorSetUpdate).to.be.undefined();
  });

  it('should rotate validator set and return ValidatorSetUpdate if height is divisible by ROTATION_BLOCK_INTERVAL', async () => {
    height = Long.fromInt(15);

    validatorSetMock.rotate.resolves(true);

    const quorumHash = Buffer.alloc(64).fill(1).toString('hex');
    validatorSetMock.getQuorum.returns({
      quorumHash,
    });

    const validatorSetUpdate = new ValidatorSetUpdate();

    createValidatorSetUpdateMock.returns(validatorSetUpdate);

    const response = await endBlock(height, loggerMock);

    expect(validatorSetMock.rotate).to.be.calledOnceWithExactly(
      height,
      chainLockMock.height,
      Buffer.from(lastCommitInfoMock.stateSignature),
    );

    expect(createValidatorSetUpdateMock).to.be.calledOnceWithExactly(validatorSetMock);

    expect(response.validatorSetUpdate).to.be.equal(validatorSetUpdate);
  });

  it('should return consensusParamUpdates if request contains update consensus features flag', async function it() {
    const getLatestFeatureFlagGetMock = this.sinon.stub();
    getLatestFeatureFlagGetMock.withArgs('block').returns({
      maxBytes: 1,
      maxGas: 2,
    });
    getLatestFeatureFlagGetMock.withArgs('evidence').returns({
      maxAgeNumBlocks: 1,
      maxAgeDuration: null,
      maxBytes: 2,
    });
    getLatestFeatureFlagGetMock.withArgs('version').returns({
      appVersion: 1,
    });

    getFeatureFlagForHeightMock.resolves({
      get: getLatestFeatureFlagGetMock,
    });

    const response = await endBlock(height, loggerMock);

    expect(response).to.deep.equal({
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
    });

    expect(getFeatureFlagForHeightMock).to.be.calledOnce();
  });
});
