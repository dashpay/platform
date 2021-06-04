const {
  tendermint: {
    abci: {
      ResponseEndBlock,
      ValidatorSetUpdate,
    },
    types: {
      CoreChainLock,
    },
  },
} = require('@dashevo/abci/types');

const generateRandomIdentifier = require('@dashevo/dpp/lib/test/utils/generateRandomIdentifier');

const endBlockHandlerFactory = require('../../../../lib/abci/handlers/endBlockHandlerFactory');

const BlockExecutionContextMock = require('../../../../lib/test/mock/BlockExecutionContextMock');

const NoDPNSContractFoundError = require('../../../../lib/abci/handlers/errors/NoDPNSContractFoundError');
const NoDashpayContractFoundError = require('../../../../lib/abci/handlers/errors/NoDashpayContractFoundError');
const LoggerMock = require('../../../../lib/test/mock/LoggerMock');

describe('endBlockHandlerFactory', () => {
  let endBlockHandler;
  let requestMock;
  let headerMock;
  let lastCommitInfoMock;
  let blockExecutionContextMock;
  let dpnsContractId;
  let dpnsContractBlockHeight;
  let dashpayContractId;
  let dashpayContractBlockHeight;
  let latestCoreChainLockMock;
  let loggerMock;
  let createValidatorSetUpdateMock;
  let chainLockMock;
  let validatorSetMock;

  beforeEach(function beforeEach() {
    headerMock = {
      coreChainLockedHeight: 2,
    };

    lastCommitInfoMock = {
      stateSignature: Uint8Array.from('003657bb44d74c371d14485117de43313ca5c2848f3622d691c2b1bf3576a64bdc2538efab24854eb82ae7db38482dbd15a1cb3bc98e55173817c9d05c86e47a5d67614a501414aae6dd1565e59422d1d77c41ae9b38de34ecf1e9f778b2a97b'),
    };

    blockExecutionContextMock = new BlockExecutionContextMock(this.sinon);

    blockExecutionContextMock.hasDataContract.returns(true);
    blockExecutionContextMock.getHeader.returns(headerMock);
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

    dpnsContractId = generateRandomIdentifier();
    dpnsContractBlockHeight = 2;

    dashpayContractId = generateRandomIdentifier();
    dashpayContractBlockHeight = 2;

    validatorSetMock = {
      rotate: this.sinon.stub(),
      getQuorum: this.sinon.stub(),
    };

    createValidatorSetUpdateMock = this.sinon.stub();

    endBlockHandler = endBlockHandlerFactory(
      blockExecutionContextMock,
      dpnsContractBlockHeight,
      dpnsContractId,
      dashpayContractBlockHeight,
      dashpayContractId,
      latestCoreChainLockMock,
      validatorSetMock,
      createValidatorSetUpdateMock,
      loggerMock,
    );

    requestMock = {
      height: dpnsContractBlockHeight,
    };
  });

  it('should return a response', async () => {
    endBlockHandler = endBlockHandlerFactory(
      blockExecutionContextMock,
      undefined,
      undefined,
      undefined,
      undefined,
      latestCoreChainLockMock,
      validatorSetMock,
      createValidatorSetUpdateMock,
      loggerMock,
    );

    const response = await endBlockHandler(requestMock);

    expect(response).to.be.an.instanceOf(ResponseEndBlock);
    expect(response.toJSON()).to.be.empty();

    expect(blockExecutionContextMock.hasDataContract).to.not.have.been.called();
  });

  it('should return a response if DPNS contract is present at specified height', async () => {
    endBlockHandler = endBlockHandlerFactory(
      blockExecutionContextMock,
      dpnsContractBlockHeight,
      dpnsContractId,
      undefined,
      undefined,
      latestCoreChainLockMock,
      validatorSetMock,
      createValidatorSetUpdateMock,
      loggerMock,
    );

    const response = await endBlockHandler(requestMock);

    expect(response).to.be.an.instanceOf(ResponseEndBlock);

    expect(response.toJSON()).to.be.empty();

    expect(blockExecutionContextMock.hasDataContract).to.have.been.calledOnceWithExactly(
      dpnsContractId,
    );
  });

  it('should throw and error if DPNS contract is not present at specified height', async () => {
    endBlockHandler = endBlockHandlerFactory(
      blockExecutionContextMock,
      dpnsContractBlockHeight,
      dpnsContractId,
      undefined,
      undefined,
      latestCoreChainLockMock,
      validatorSetMock,
      createValidatorSetUpdateMock,
      loggerMock,
    );

    blockExecutionContextMock.hasDataContract.returns(false);

    try {
      await endBlockHandler(requestMock);

      expect.fail('Error was not thrown');
    } catch (e) {
      expect(e).to.be.an.instanceOf(NoDPNSContractFoundError);
      expect(e.getContractId()).to.equal(dpnsContractId);
      expect(e.getHeight()).to.equal(dpnsContractBlockHeight);

      expect(blockExecutionContextMock.hasDataContract).to.have.been.calledOnceWithExactly(
        dpnsContractId,
      );

      expect(latestCoreChainLockMock.getChainLock).to.have.not.been.called();
    }
  });

  it('should return a response if DashPay contract is present at specified height', async () => {
    endBlockHandler = endBlockHandlerFactory(
      blockExecutionContextMock,
      undefined,
      undefined,
      dashpayContractBlockHeight,
      dashpayContractId,
      latestCoreChainLockMock,
      validatorSetMock,
      createValidatorSetUpdateMock,
      loggerMock,
    );

    const response = await endBlockHandler(requestMock);

    expect(response).to.be.an.instanceOf(ResponseEndBlock);

    expect(response.toJSON()).to.be.empty();

    expect(blockExecutionContextMock.hasDataContract).to.have.been.calledOnceWithExactly(
      dashpayContractId,
    );
  });

  it('should throw and error if DashPay contract is not present at specified height', async () => {
    endBlockHandler = endBlockHandlerFactory(
      blockExecutionContextMock,
      undefined,
      undefined,
      dashpayContractBlockHeight,
      dashpayContractId,
      latestCoreChainLockMock,
      validatorSetMock,
      createValidatorSetUpdateMock,
      loggerMock,
    );

    blockExecutionContextMock.hasDataContract.returns(false);

    try {
      await endBlockHandler(requestMock);

      expect.fail('Error was not thrown');
    } catch (e) {
      expect(e).to.be.an.instanceOf(NoDashpayContractFoundError);
      expect(e.getContractId()).to.equal(dashpayContractId);
      expect(e.getHeight()).to.equal(dashpayContractBlockHeight);

      expect(blockExecutionContextMock.hasDataContract).to.have.been.calledOnceWithExactly(
        dashpayContractId,
      );

      expect(latestCoreChainLockMock.getChainLock).to.have.not.been.called();
    }
  });

  it('should return nextCoreChainLockUpdate if latestCoreChainLock above header height', async () => {
    chainLockMock.height = 3;

    const response = await endBlockHandler(requestMock);

    expect(latestCoreChainLockMock.getChainLock).to.have.been.calledOnceWithExactly();

    const expectedCoreChainLock = new CoreChainLock({
      coreBlockHeight: chainLockMock.height,
      coreBlockHash: chainLockMock.blockHash,
      signature: chainLockMock.signature,
    });

    expect(response.nextCoreChainLockUpdate).to.deep.equal(expectedCoreChainLock);
    expect(response.validatorSetUpdate).to.be.null();
  });

  it('should rotate validator set and return ValidatorSetUpdate if height is divisible by ROTATION_BLOCK_INTERVAL', async () => {
    requestMock = {
      height: 15,
    };

    validatorSetMock.rotate.resolves(true);

    const quorumHash = Buffer.alloc(64).fill(1).toString('hex');
    validatorSetMock.getQuorum.returns({
      quorumHash,
    });

    const validatorSetUpdate = new ValidatorSetUpdate();

    createValidatorSetUpdateMock.returns(validatorSetUpdate);

    const response = await endBlockHandler(requestMock);

    expect(response).to.be.an.instanceOf(ResponseEndBlock);

    expect(validatorSetMock.rotate).to.be.calledOnceWithExactly(
      requestMock.height,
      chainLockMock.height,
      Buffer.from(lastCommitInfoMock.stateSignature),
    );

    expect(createValidatorSetUpdateMock).to.be.calledOnceWithExactly(validatorSetMock);

    expect(response.validatorSetUpdate).to.be.equal(validatorSetUpdate);
  });
});
