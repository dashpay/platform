const Long = require('long');

const {
  tendermint: {
    abci: {
      ResponseBeginBlock,
    },
  },
} = require('@dashevo/abci/types');

const beginBlockHandlerFactory = require('../../../../lib/abci/handlers/beginBlockHandlerFactory');

const BlockExecutionDBTransactionsMock = require('../../../../lib/test/mock/BlockExecutionStoreTransactionsMock');
const BlockExecutionContextMock = require('../../../../lib/test/mock/BlockExecutionContextMock');
const LoggerMock = require('../../../../lib/test/mock/LoggerMock');
const NotSupportedNetworkProtocolVersionError = require('../../../../lib/abci/handlers/errors/NotSupportedProtocolVersionError');
const NetworkProtocolVersionIsNotSetError = require('../../../../lib/abci/handlers/errors/NetworkProtocolVersionIsNotSetError');

describe('beginBlockHandlerFactory', () => {
  let protocolVersion;
  let beginBlockHandler;
  let request;
  let blockHeight;
  let coreChainLockedHeight;
  let blockExecutionDBTransactionsMock;
  let blockExecutionContextMock;
  let previousBlockExecutionContextMock;
  let header;
  let updateSimplifiedMasternodeListMock;
  let waitForChainLockedHeightMock;
  let loggerMock;
  let lastCommitInfo;
  let dppMock;
  let transactionalDppMock;

  beforeEach(function beforeEach() {
    protocolVersion = Long.fromInt(1);

    blockExecutionDBTransactionsMock = new BlockExecutionDBTransactionsMock(this.sinon);

    blockExecutionContextMock = new BlockExecutionContextMock(this.sinon);
    previousBlockExecutionContextMock = new BlockExecutionContextMock(this.sinon);

    loggerMock = new LoggerMock(this.sinon);

    dppMock = {
      setProtocolVersion: this.sinon.stub(),
    };
    transactionalDppMock = {
      setProtocolVersion: this.sinon.stub(),
    };

    updateSimplifiedMasternodeListMock = this.sinon.stub();
    waitForChainLockedHeightMock = this.sinon.stub();

    beginBlockHandler = beginBlockHandlerFactory(
      blockExecutionDBTransactionsMock,
      blockExecutionContextMock,
      previousBlockExecutionContextMock,
      protocolVersion,
      dppMock,
      transactionalDppMock,
      updateSimplifiedMasternodeListMock,
      waitForChainLockedHeightMock,
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

  it('should update height, start transactions return ResponseBeginBlock', async () => {
    const response = await beginBlockHandler(request);

    expect(response).to.be.an.instanceOf(ResponseBeginBlock);

    expect(blockExecutionDBTransactionsMock.start).to.be.calledOnceWithExactly();

    expect(previousBlockExecutionContextMock.reset).to.be.calledOnceWithExactly();
    expect(previousBlockExecutionContextMock.populate).to.be.calledOnceWithExactly(
      blockExecutionContextMock,
    );

    expect(blockExecutionContextMock.reset).to.be.calledOnceWithExactly();
    expect(blockExecutionContextMock.setHeader).to.be.calledOnceWithExactly(header);
    expect(blockExecutionContextMock.setLastCommitInfo).to.be.calledOnceWithExactly(lastCommitInfo);

    expect(updateSimplifiedMasternodeListMock).to.be.calledOnceWithExactly(
      coreChainLockedHeight, { logger: loggerMock },
    );
    expect(waitForChainLockedHeightMock).to.be.calledOnceWithExactly(coreChainLockedHeight);
    expect(blockExecutionDBTransactionsMock.abort).to.be.not.called();
    expect(dppMock.setProtocolVersion).to.have.been.calledOnceWithExactly(
      protocolVersion.toNumber(),
    );
    expect(transactionalDppMock.setProtocolVersion).to.have.been.calledOnceWithExactly(
      protocolVersion.toNumber(),
    );
  });

  it('should throw NotSupportedProtocolVersionError if protocol version is not supported', async () => {
    request.header.version.app = Long.fromInt(42);

    try {
      await beginBlockHandler(request);

      expect.fail('should throw NotSupportedProtocolVersionError');
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

  it('should abort already started transactions', async () => {
    blockExecutionDBTransactionsMock.isStarted.returns(true);

    const response = await beginBlockHandler(request);

    expect(response).to.be.an.instanceOf(ResponseBeginBlock);

    expect(blockExecutionDBTransactionsMock.start).to.be.calledOnceWithExactly();

    expect(previousBlockExecutionContextMock.reset).to.be.calledOnceWithExactly();
    expect(previousBlockExecutionContextMock.populate).to.be.calledOnceWithExactly(
      blockExecutionContextMock,
    );

    expect(blockExecutionContextMock.reset).to.be.calledOnceWithExactly();
    expect(blockExecutionContextMock.setHeader).to.be.calledOnceWithExactly(header);
    expect(blockExecutionContextMock.setLastCommitInfo).to.be.calledOnceWithExactly(lastCommitInfo);

    expect(updateSimplifiedMasternodeListMock).to.be.calledOnceWithExactly(
      coreChainLockedHeight, { logger: loggerMock },
    );

    expect(waitForChainLockedHeightMock).to.be.calledOnceWithExactly(coreChainLockedHeight);
    expect(blockExecutionDBTransactionsMock.abort).to.be.calledOnce();
  });
});
