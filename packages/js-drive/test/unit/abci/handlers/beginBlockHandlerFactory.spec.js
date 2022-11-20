const Long = require('long');

const {
  tendermint: {
    abci: {
      ResponseBeginBlock,
    },
  },
} = require('@dashevo/abci/types');

const beginBlockHandlerFactory = require('../../../../lib/abci/handlers/beginBlockHandlerFactory');

const BlockExecutionContextMock = require('../../../../lib/test/mock/BlockExecutionContextMock');
const LoggerMock = require('../../../../lib/test/mock/LoggerMock');
const NotSupportedNetworkProtocolVersionError = require('../../../../lib/abci/handlers/errors/NotSupportedNetworkProtocolVersionError');
const NetworkProtocolVersionIsNotSetError = require('../../../../lib/abci/handlers/errors/NetworkProtocolVersionIsNotSetError');
const GroveDBStoreMock = require('../../../../lib/test/mock/GroveDBStoreMock');
const BlockExecutionContextStackMock = require('../../../../lib/test/mock/BlockExecutionContextStackMock');
const millisToProtoTimestamp = require('../../../../lib/util/millisToProtoTimestamp');
const BlockInfo = require('../../../../lib/blockExecution/BlockInfo');

describe('beginBlockHandlerFactory', () => {
  let protocolVersion;
  let beginBlockHandler;
  let request;
  let blockHeight;
  let coreChainLockedHeight;
  let blockExecutionContextMock;
  let header;
  let updateSimplifiedMasternodeListMock;
  let waitForChainLockedHeightMock;
  let loggerMock;
  let lastCommitInfo;
  let dppMock;
  let transactionalDppMock;
  let synchronizeMasternodeIdentitiesMock;
  let groveDBStoreMock;
  let blockExecutionContextStackMock;
  let executionTimerMock;
  let rsAbciMock;
  let rsResponseMock;
  let blockInfo;
  let timeMs;

  beforeEach(function beforeEach() {
    protocolVersion = Long.fromInt(1);

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

    blockExecutionContextMock = new BlockExecutionContextMock(this.sinon);

    executionTimerMock = {
      clearTimer: this.sinon.stub(),
      startTimer: this.sinon.stub(),
      stopTimer: this.sinon.stub(),
    };

    rsResponseMock = {
      epochInfo: {
        currentEpochIndex: 1,
        isEpochChange: false,
      },
    };

    rsAbciMock = {
      blockBegin: this.sinon.stub().resolves(rsResponseMock),
    };

    beginBlockHandler = beginBlockHandlerFactory(
      groveDBStoreMock,
      blockExecutionContextMock,
      blockExecutionContextStackMock,
      protocolVersion,
      dppMock,
      transactionalDppMock,
      updateSimplifiedMasternodeListMock,
      waitForChainLockedHeightMock,
      synchronizeMasternodeIdentitiesMock,
      loggerMock,
      executionTimerMock,
      rsAbciMock,
    );

    blockHeight = Long.fromNumber(1);

    timeMs = Date.now();

    header = {
      version: {
        app: protocolVersion,
      },
      height: blockHeight,
      time: millisToProtoTimestamp(timeMs),
      coreChainLockedHeight,
      proposerProTxHash: Buffer.alloc(32, 1),
    };

    blockInfo = new BlockInfo(
      blockHeight.toNumber(),
      rsResponseMock.epochInfo.currentEpochIndex,
      timeMs,
    );

    blockExecutionContextMock.getHeader.returns(header);
    blockExecutionContextMock.getEpochInfo.returns(rsResponseMock.epochInfo);
    blockExecutionContextMock.getTimeMs.returns(timeMs);

    lastCommitInfo = {};

    request = {
      header,
      lastCommitInfo,
    };
  });

  it('should reset previous block state and prepare everything for for a next one', async () => {
    const response = await beginBlockHandler(request);

    expect(response).to.be.an.instanceOf(ResponseBeginBlock);

    // Wait for chain locked core block height
    expect(waitForChainLockedHeightMock).to.be.calledOnceWithExactly(coreChainLockedHeight);

    // Reset block execution context
    expect(blockExecutionContextMock.reset).to.be.calledOnceWithExactly();
    expect(blockExecutionContextMock.setHeader).to.be.calledOnceWithExactly(header);
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

    const response = await beginBlockHandler(request);

    expect(response).to.be.an.instanceOf(ResponseBeginBlock);

    expect(synchronizeMasternodeIdentitiesMock).to.have.been.calledOnceWithExactly(
      coreChainLockedHeight,
      blockInfo,
    );
  });

  it('should throw NotSupportedNetworkProtocolVersionError if protocol version is not supported', async () => {
    request.header.version.app = Long.fromInt(42);

    try {
      await beginBlockHandler(request);

      expect.fail('should throw NotSupportedNetworkProtocolVersionError');
    } catch (e) {
      expect(e).to.be.instanceOf(NotSupportedNetworkProtocolVersionError);
      expect(e.getNetworkProtocolVersion()).to.equal(request.header.version.app);
      expect(e.getLatestProtocolVersion()).to.equal(protocolVersion);
    }
  });

  it('should throw an NetworkProtocolVersionIsNotSetError if network protocol version is not set', async () => {
    request.header.version.app = Long.fromInt(0);

    try {
      await beginBlockHandler(request);

      expect.fail('should throw NetworkProtocolVersionIsNotSetError');
    } catch (err) {
      expect(err).to.be.an.instanceOf(NetworkProtocolVersionIsNotSetError);
    }
  });

  it('should abort db transaction and reset previous execution context if previous block failed', async function it() {
    blockExecutionContextMock.getHeader.returns({
      height: {
        equals: this.sinon.stub().returns(true),
      },
      time: millisToProtoTimestamp(timeMs),
    });

    blockExecutionContextStackMock.getFirst.returns({
      getHeader: this.sinon.stub().returns(
        {
          height: {
            equals: this.sinon.stub().returns(true),
            toNumber: this.sinon.stub().returns(1000),
          },
        },
      ),
      getTimeMs: this.sinon.stub().returns(timeMs),
    });

    groveDBStoreMock.isTransactionStarted.resolves(true);

    const response = await beginBlockHandler(request);

    expect(response).to.be.an.instanceOf(ResponseBeginBlock);

    expect(groveDBStoreMock.abortTransaction).to.be.calledOnceWithExactly();
    expect(blockExecutionContextStackMock.removeFirst).to.be.calledOnceWithExactly();

    expect(blockExecutionContextMock.reset).to.be.calledOnceWithExactly();
    expect(blockExecutionContextMock.setHeader).to.be.calledOnceWithExactly(header);
    expect(blockExecutionContextMock.setLastCommitInfo).to.be.calledOnceWithExactly(lastCommitInfo);

    expect(updateSimplifiedMasternodeListMock).to.be.calledOnceWithExactly(
      coreChainLockedHeight, { logger: loggerMock },
    );

    expect(waitForChainLockedHeightMock).to.be.calledOnceWithExactly(coreChainLockedHeight);
    expect(synchronizeMasternodeIdentitiesMock).to.have.not.been.called();
  });
});
