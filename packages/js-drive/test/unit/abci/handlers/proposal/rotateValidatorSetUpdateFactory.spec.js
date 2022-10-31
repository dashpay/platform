const {
  tendermint: {
    abci: {
      ValidatorSetUpdate,
    },
  },
} = require('@dashevo/abci/types');
const Long = require('long');
const rotateValidatorSetUpdateFactory = require('../../../../../lib/abci/handlers/proposal/rotateValidatorSetUpdateFactory');
const BlockExecutionContextMock = require('../../../../../lib/test/mock/BlockExecutionContextMock');
const LoggerMock = require('../../../../../lib/test/mock/LoggerMock');

describe('rotateValidatorSetUpdateFactory.spec', () => {
  let rotateValidatorSetUpdate;
  let blockExecutionContextMock;
  let validatorSetMock;
  let createValidatorSetUpdateMock;
  let latestCoreChainLockMock;
  let chainLockMock;
  let height;
  let loggerMock;
  let lastCommitInfoMock;
  let round;
  let proposalBlockExecutionContextCollectionMock;

  beforeEach(function beforeEach() {
    round = 0;
    chainLockMock = {
      height: 1,
      blockHash: Buffer.alloc(0),
      signature: Buffer.alloc(0),
    };

    lastCommitInfoMock = {
      stateSignature: Uint8Array.from('003657bb44d74c371d14485117de43313ca5c2848f3622d691c2b1bf3576a64bdc2538efab24854eb82ae7db38482dbd15a1cb3bc98e55173817c9d05c86e47a5d67614a501414aae6dd1565e59422d1d77c41ae9b38de34ecf1e9f778b2a97b'),
    };

    blockExecutionContextMock = new BlockExecutionContextMock(this.sinon);
    blockExecutionContextMock.getLastCommitInfo.returns(lastCommitInfoMock);

    validatorSetMock = {
      rotate: this.sinon.stub(),
      getQuorum: this.sinon.stub(),
    };

    createValidatorSetUpdateMock = this.sinon.stub();

    latestCoreChainLockMock = {
      getChainLock: this.sinon.stub().returns(chainLockMock),
    };

    loggerMock = new LoggerMock(this.sinon);
    proposalBlockExecutionContextCollectionMock = {
      get: this.sinon.stub().returns(blockExecutionContextMock),
    };

    rotateValidatorSetUpdate = rotateValidatorSetUpdateFactory(
      proposalBlockExecutionContextCollectionMock,
      validatorSetMock,
      createValidatorSetUpdateMock,
      latestCoreChainLockMock,
    );
  });

  it('should rotate validator set and return ValidatorSetUpdate if height is divisible by ROTATION_BLOCK_INTERVAL', async () => {
    height = Long.fromInt(15);

    const quorumHash = Buffer.alloc(64).fill(1).toString('hex');

    validatorSetMock.rotate.resolves(true);
    validatorSetMock.getQuorum.resolves({ quorumHash });

    const validatorSetUpdate = new ValidatorSetUpdate();

    createValidatorSetUpdateMock.returns(validatorSetUpdate);

    const response = await rotateValidatorSetUpdate(height, round, loggerMock);

    expect(validatorSetMock.rotate).to.be.calledOnceWithExactly(
      height,
      chainLockMock.height,
      Buffer.from(lastCommitInfoMock.stateSignature),
    );

    expect(createValidatorSetUpdateMock).to.be.calledOnceWithExactly(validatorSetMock);
    expect(proposalBlockExecutionContextCollectionMock.get).to.have.been.calledOnceWithExactly(
      round,
    );
    expect(response).to.be.equal(validatorSetUpdate);
  });

  it('should return undefined', async () => {
    height = Long.fromInt(15);

    validatorSetMock.rotate.resolves(false);

    const response = await rotateValidatorSetUpdate(height, round, loggerMock);

    expect(validatorSetMock.rotate).to.be.calledOnceWithExactly(
      height,
      chainLockMock.height,
      Buffer.from(lastCommitInfoMock.stateSignature),
    );

    expect(createValidatorSetUpdateMock).to.not.be.called();
    expect(proposalBlockExecutionContextCollectionMock.get).to.have.been.calledOnceWithExactly(
      round,
    );
    expect(response).to.be.undefined();
  });
});
