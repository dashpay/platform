const Long = require('long');

const {
  tendermint: {
    abci: {
      ResponseInitChain,
      ValidatorSetUpdate,
    },
    types: {
      CoreChainLock,
    },
  },
} = require('@dashevo/abci/types');

const initChainHandlerFactory = require('../../../../lib/abci/handlers/initChainHandlerFactory');
const LoggerMock = require('../../../../lib/test/mock/LoggerMock');
const GroveDBStoreMock = require('../../../../lib/test/mock/GroveDBStoreMock');
const protoTimestampToMillis = require('../../../../lib/util/protoTimestampToMillis');
const millisToProtoTimestamp = require('../../../../lib/util/millisToProtoTimestamp');
const BlockInfo = require('../../../../lib/blockExecution/BlockInfo');

describe('initChainHandlerFactory', () => {
  let initChainHandler;
  let updateSimplifiedMasternodeListMock;
  let initialCoreChainLockedHeight;
  let validatorSetMock;
  let createValidatorSetUpdateMock;
  let loggerMock;
  let validatorSetUpdate;
  let synchronizeMasternodeIdentitiesMock;
  let registerSystemDataContractsMock;
  let groveDBStoreMock;
  let appHashFixture;
  let rsAbciMock;
  let createCoreChainLockUpdateMock;
  let coreChainLockUpdate;
  let createContextLoggerMock;

  beforeEach(function beforeEach() {
    initialCoreChainLockedHeight = 1;

    appHashFixture = Buffer.alloc(0);

    updateSimplifiedMasternodeListMock = this.sinon.stub();

    const quorumHash = Buffer.alloc(64).fill(1).toString('hex');
    validatorSetMock = {
      initialize: this.sinon.stub(),
      getQuorum: this.sinon.stub().returns({
        quorumHash,
      }),
    };

    validatorSetUpdate = new ValidatorSetUpdate();

    createValidatorSetUpdateMock = this.sinon.stub().returns(validatorSetUpdate);
    synchronizeMasternodeIdentitiesMock = this.sinon.stub().resolves({
      createdEntities: [],
      updatedEntities: [],
      removedEntities: [],
      fromHeight: 1,
      toHeight: 42,
    });

    loggerMock = new LoggerMock(this.sinon);

    registerSystemDataContractsMock = this.sinon.stub();
    rsAbciMock = {
      initChain: this.sinon.stub(),
    };

    groveDBStoreMock = new GroveDBStoreMock(this.sinon);
    groveDBStoreMock.getRootHash.resolves(appHashFixture);

    coreChainLockUpdate = new CoreChainLock({
      coreBlockHeight: 42,
      coreBlockHash: '1528e523f4c20fa84ba70dd96372d34e00ce260f357d53ad1a8bc892ebf20e2d',
      signature: '1897ce8f54d2070f44ca5c29983b68b391e8137c25e44f67416e579f3e3bdfef7b4fd22db7818399147e52907998857b0fbc8edfdc40a64f2c7df0e88544d31d12ca8c15e73d50dda25ca23f754ed3f789ed4bcb392161995f464017c10df404',
    });

    createCoreChainLockUpdateMock = this.sinon.stub().resolves(coreChainLockUpdate);
    createContextLoggerMock = this.sinon.stub().returns(loggerMock);

    initChainHandler = initChainHandlerFactory(
      updateSimplifiedMasternodeListMock,
      initialCoreChainLockedHeight,
      validatorSetMock,
      createValidatorSetUpdateMock,
      synchronizeMasternodeIdentitiesMock,
      loggerMock,
      registerSystemDataContractsMock,
      groveDBStoreMock,
      rsAbciMock,
      createCoreChainLockUpdateMock,
      createContextLoggerMock,
    );
  });

  it('should initialize the chain', async () => {
    const request = {
      initialHeight: Long.fromInt(1),
      chainId: 'test',
      time: millisToProtoTimestamp(Date.now()),
    };

    const blockInfo = new BlockInfo(0, 0, protoTimestampToMillis(request.time));

    const response = await initChainHandler(request);

    expect(response).to.be.an.instanceOf(ResponseInitChain);
    expect(response.validatorSetUpdate).to.be.equal(validatorSetUpdate);
    expect(response.initialCoreHeight).to.be.equal(initialCoreChainLockedHeight);
    expect(response.appHash).to.deep.equal(appHashFixture);

    // Update SML

    expect(updateSimplifiedMasternodeListMock).to.be.calledOnceWithExactly(
      initialCoreChainLockedHeight,
      {
        logger: loggerMock,
      },
    );

    // Create initial state

    expect(groveDBStoreMock.startTransaction).to.be.calledOnce();

    expect(rsAbciMock.initChain).to.be.calledOnceWithExactly({}, true);

    expect(registerSystemDataContractsMock).to.be.calledOnceWithExactly(loggerMock, blockInfo);

    expect(synchronizeMasternodeIdentitiesMock).to.be.calledOnceWithExactly(
      initialCoreChainLockedHeight,
      blockInfo,
    );

    expect(groveDBStoreMock.commitTransaction).to.be.calledOnce();

    expect(groveDBStoreMock.getRootHash).to.be.calledOnce();

    // Initialize VS

    expect(validatorSetMock.initialize).to.be.calledOnceWithExactly(
      initialCoreChainLockedHeight,
    );

    expect(validatorSetMock.getQuorum).to.be.calledOnce();

    expect(createValidatorSetUpdateMock).to.be.calledOnceWithExactly(validatorSetMock);

    expect(createCoreChainLockUpdateMock)
      .to.be.calledOnceWithExactly(initialCoreChainLockedHeight, 0, loggerMock);
    expect(createContextLoggerMock).to.be.calledOnceWithExactly(loggerMock, {
      height: request.initialHeight.toString(),
      abciMethod: 'initChain',
    });
  });
});
