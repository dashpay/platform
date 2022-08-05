const Long = require('long');

const beginBlockFactory = require('../../../../../lib/abci/handlers/finalizeBlock/beginBlockFactory');

const BlockExecutionContextMock = require('../../../../../lib/test/mock/BlockExecutionContextMock');
const LoggerMock = require('../../../../../lib/test/mock/LoggerMock');
const NotSupportedNetworkProtocolVersionError = require('../../../../../lib/abci/handlers/errors/NotSupportedNetworkProtocolVersionError');
const NetworkProtocolVersionIsNotSetError = require('../../../../../lib/abci/handlers/errors/NetworkProtocolVersionIsNotSetError');
const GroveDBStoreMock = require('../../../../../lib/test/mock/GroveDBStoreMock');
const BlockExecutionContextStackMock = require('../../../../../lib/test/mock/BlockExecutionContextStackMock');

describe('beginBlockFactory', () => {
  let protocolVersion;
  let beginBlock;
  let request;
  let blockHeight;
  let coreChainLockedHeight;
  let blockExecutionContextMock;
  let updateSimplifiedMasternodeListMock;
  let waitForChainLockedHeightMock;
  let loggerMock;
  let lastCommitInfo;
  let dppMock;
  let transactionalDppMock;
  let synchronizeMasternodeIdentitiesMock;
  let groveDBStoreMock;
  let blockExecutionContextStackMock;
  let version;
  let time;

  beforeEach(function beforeEach() {
    protocolVersion = Long.fromInt(1);

    blockExecutionContextMock = new BlockExecutionContextMock(this.sinon);

    loggerMock = new LoggerMock(this.sinon);

    dppMock = {
      setProtocolVersion: this.sinon.stub(),
    };
    transactionalDppMock = {
      setProtocolVersion: this.sinon.stub(),
    };

    updateSimplifiedMasternodeListMock = this.sinon.stub().resolves(false);
    waitForChainLockedHeightMock = this.sinon.stub();
    synchronizeMasternodeIdentitiesMock = this.sinon.stub().resolves({
      createdEntities: [],
      updatedEntities: [],
      removedEntities: [],
      fromHeight: 1,
      toHeight: 42,
    });

    groveDBStoreMock = new GroveDBStoreMock(this.sinon);
    blockExecutionContextStackMock = new BlockExecutionContextStackMock(this.sinon);

    blockExecutionContextStackMock.getLatest.returns({
      getHeader: this.sinon.stub(),
    });

    beginBlock = beginBlockFactory(
      groveDBStoreMock,
      blockExecutionContextMock,
      blockExecutionContextStackMock,
      protocolVersion,
      dppMock,
      transactionalDppMock,
      updateSimplifiedMasternodeListMock,
      waitForChainLockedHeightMock,
      synchronizeMasternodeIdentitiesMock,
    );

    blockHeight = 2;
    blockHeight = 1;

    lastCommitInfo = {};
    version = {
      app: protocolVersion,
    };

    time = {
      seconds: Math.ceil(new Date().getTime() / 1000),
    };

    request = {
      height: blockHeight,
      lastCommitInfo,
      coreChainLockedHeight,
      version,
      time,
    };
  });

  it('should reset previous block state and prepare everything for for a next one', async () => {
    await beginBlock(request, loggerMock);

    // Wait for chain locked core block height
    expect(waitForChainLockedHeightMock).to.be.calledOnceWithExactly(coreChainLockedHeight);

    // Reset block execution context
    expect(blockExecutionContextMock.getHeight).to.be.calledOnceWithExactly();
    expect(blockExecutionContextMock.reset).to.be.calledOnceWithExactly();
    expect(blockExecutionContextMock.setConsensusLogger).to.be.calledOnceWithExactly(loggerMock);
    expect(blockExecutionContextMock.setHeight).to.be.calledOnceWithExactly(blockHeight);
    expect(blockExecutionContextMock.setVersion).to.be.calledOnceWithExactly(version);
    expect(blockExecutionContextMock.setTime).to.be.calledOnceWithExactly(time);
    expect(blockExecutionContextMock.setCoreChainLockedHeight).to.be.calledOnceWithExactly(
      coreChainLockedHeight,
    );
    expect(blockExecutionContextMock.setLastCommitInfo).to.be.calledOnceWithExactly(lastCommitInfo);

    // Set current protocol version
    expect(dppMock.setProtocolVersion).to.have.been.calledOnceWithExactly(
      protocolVersion.toNumber(),
    );
    expect(transactionalDppMock.setProtocolVersion).to.have.been.calledOnceWithExactly(
      protocolVersion.toNumber(),
    );

    // Start new transaction
    expect(groveDBStoreMock.startTransaction).to.be.calledOnceWithExactly();

    // Update SML
    expect(updateSimplifiedMasternodeListMock).to.be.calledOnceWithExactly(
      coreChainLockedHeight, { logger: loggerMock },
    );

    expect(synchronizeMasternodeIdentitiesMock).to.not.been.called();
  });

  it('should synchronize masternode identities if SML is updated', async () => {
    updateSimplifiedMasternodeListMock.resolves(true);

    await beginBlock(request, loggerMock);

    expect(synchronizeMasternodeIdentitiesMock).to.have.been.calledOnceWithExactly(
      coreChainLockedHeight,
    );
  });

  it('should throw NotSupportedNetworkProtocolVersionError if protocol version is not supported', async () => {
    request.version.app = Long.fromInt(42);

    try {
      await beginBlock(request, loggerMock);

      expect.fail('should throw NotSupportedNetworkProtocolVersionError');
    } catch (e) {
      expect(e).to.be.instanceOf(NotSupportedNetworkProtocolVersionError);
      expect(e.getNetworkProtocolVersion()).to.equal(request.version.app);
      expect(e.getLatestProtocolVersion()).to.equal(protocolVersion);
    }
  });

  it('should throw an NetworkProtocolVersionIsNotSetError if network protocol version is not set', async () => {
    request.version.app = Long.fromInt(0);

    try {
      await beginBlock(request, loggerMock);

      expect.fail('should throw NetworkProtocolVersionIsNotSetError');
    } catch (err) {
      expect(err).to.be.an.instanceOf(NetworkProtocolVersionIsNotSetError);
    }
  });

  it('should abort db transaction and reset previous execution context if previous block failed', async function it() {
    blockExecutionContextMock.getHeight.returns({
      equals: this.sinon.stub().returns(true),
    });

    blockExecutionContextStackMock.getLatest.returns({
      getHeight: this.sinon.stub().returns(
        {
          equals: this.sinon.stub().returns(true),
        },
      ),
    });

    groveDBStoreMock.isTransactionStarted.resolves(true);

    await beginBlock(request, loggerMock);

    expect(groveDBStoreMock.abortTransaction).to.be.calledOnceWithExactly();
    expect(blockExecutionContextStackMock.removeLatest).to.be.calledOnceWithExactly();

    expect(blockExecutionContextMock.reset).to.be.calledOnceWithExactly();
    expect(blockExecutionContextMock.setConsensusLogger).to.be.calledOnceWithExactly(loggerMock);
    expect(blockExecutionContextMock.setHeight).to.be.calledOnceWithExactly(blockHeight);
    expect(blockExecutionContextMock.setVersion).to.be.calledOnceWithExactly(version);
    expect(blockExecutionContextMock.setTime).to.be.calledOnceWithExactly(time);
    expect(blockExecutionContextMock.setCoreChainLockedHeight).to.be.calledOnceWithExactly(
      coreChainLockedHeight,
    );
    expect(blockExecutionContextMock.setLastCommitInfo).to.be.calledOnceWithExactly(lastCommitInfo);

    expect(updateSimplifiedMasternodeListMock).to.be.calledOnceWithExactly(
      coreChainLockedHeight, { logger: loggerMock },
    );

    expect(waitForChainLockedHeightMock).to.be.calledOnceWithExactly(coreChainLockedHeight);
    expect(synchronizeMasternodeIdentitiesMock).to.have.not.been.called();
  });
});
