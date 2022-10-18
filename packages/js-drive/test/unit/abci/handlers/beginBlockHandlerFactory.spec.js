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
  let validatorSetMock;
  let validatorSetQuorumHash;

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

    const getHeaderMock = this.sinon.stub();

    executionTimerMock = {
      clearTimer: this.sinon.stub(),
      startTimer: this.sinon.stub(),
      stopTimer: this.sinon.stub(),
    };

    rsAbciMock = {
      blockBegin: this.sinon.stub(),
    };

    validatorSetQuorumHash = Buffer.alloc(32).fill(1);

    validatorSetMock = {
      getQuorum: this.sinon.stub().returns({
        quorumHash: validatorSetQuorumHash.toString('hex'),
      }),
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
      validatorSetMock,
    );

    blockHeight = new Long(1);

    header = {
      version: {
        app: protocolVersion,
      },
      height: blockHeight,
      time: {
        seconds: Math.ceil(new Date().getTime() / 1000),
      },
      coreChainLockedHeight,
      proposerProTxHash: Buffer.alloc(32, 1),
    };

    getHeaderMock.returns(header);

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
    });

    blockExecutionContextStackMock.getFirst.returns({
      getHeader: this.sinon.stub().returns(
        {
          height: {
            equals: this.sinon.stub().returns(true),
            toNumber: this.sinon.stub().returns(1000),
          },
          time: {
            seconds: Math.ceil(new Date().getTime() / 1000),
          },
        },
      ),
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
