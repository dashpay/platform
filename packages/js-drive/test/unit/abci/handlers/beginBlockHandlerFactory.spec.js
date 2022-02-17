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
const GroveDBStoreMock = require('../../../../lib/test/mock/groveDBStoreMock');
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
    synchronizeMasternodeIdentitiesMock = this.sinon.stub();

    groveDBStoreMock = new GroveDBStoreMock(this.sinon);
    blockExecutionContextStackMock = new BlockExecutionContextStackMock(this.sinon);

    blockExecutionContextStackMock.getLatest.returns({
      getHeader: this.sinon.stub(),
    });

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
    );

    blockHeight = 2;
    blockHeight = 1;

    header = {
      version: {
        app: protocolVersion,
      },
      height: blockHeight,
      time: {
        seconds: Math.ceil(new Date().getTime() / 1000),
      },
      coreChainLockedHeight,
    };

    lastCommitInfo = {};

    request = {
      header,
      lastCommitInfo,
    };
  });

  it('should update height, update masternode identities, start transactions return ResponseBeginBlock', async () => {
    updateSimplifiedMasternodeListMock.resolves(true);
    groveDBStoreMock.isTransactionStarted.resolves(false);

    const response = await beginBlockHandler(request);

    expect(response).to.be.an.instanceOf(ResponseBeginBlock);

    expect(waitForChainLockedHeightMock).to.be.calledOnceWithExactly(coreChainLockedHeight);

    expect(blockExecutionContextMock.getHeader).to.be.calledOnceWithExactly();

    expect(blockExecutionContextMock.reset).to.be.calledOnceWithExactly();
    expect(blockExecutionContextMock.setHeader).to.be.calledOnceWithExactly(header);
    expect(blockExecutionContextMock.setLastCommitInfo).to.be.calledOnceWithExactly(lastCommitInfo);

    expect(dppMock.setProtocolVersion).to.have.been.calledOnceWithExactly(
      protocolVersion.toNumber(),
    );
    expect(transactionalDppMock.setProtocolVersion).to.have.been.calledOnceWithExactly(
      protocolVersion.toNumber(),
    );
    expect(groveDBStoreMock.isTransactionStarted).to.be.calledOnceWithExactly();
    expect(groveDBStoreMock.startTransaction).to.be.calledOnceWithExactly();

    expect(updateSimplifiedMasternodeListMock).to.be.calledOnceWithExactly(
      coreChainLockedHeight, { logger: loggerMock },
    );
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

  it('should abort already started transactions', async function it() {
    blockExecutionContextMock.getHeader.returns({
      height: {
        equals: this.sinon.stub().returns(true),
      },
    });

    blockExecutionContextStackMock.getLatest.returns({
      getHeader: this.sinon.stub().returns(
        {
          height: {
            equals: this.sinon.stub().returns(true),
          },
        },
      ),
    });
    groveDBStoreMock.isTransactionStarted.resolves(true);

    const response = await beginBlockHandler(request);

    expect(response).to.be.an.instanceOf(ResponseBeginBlock);

    expect(groveDBStoreMock.abortTransaction).to.be.calledOnceWithExactly();
    expect(blockExecutionContextStackMock.removeLatest).to.be.calledOnceWithExactly();

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
