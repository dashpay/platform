const Long = require('long');

const {
  tendermint: {
    abci: {
      ResponseInfo,
    },
  },
} = require('@dashevo/abci/types');

const infoHandlerFactory = require('../../../../lib/abci/handlers/infoHandlerFactory');

const packageJson = require('../../../../package.json');
const LoggerMock = require('../../../../lib/test/mock/LoggerMock');

const BlockExecutionContextMock = require('../../../../lib/test/mock/BlockExecutionContextMock');
const GroveDBStoreMock = require('../../../../lib/test/mock/GroveDBStoreMock');
const BlockExecutionContextRepositoryMock = require('../../../../lib/test/mock/BlockExecutionContextRepositoryMock');

describe('infoHandlerFactory', () => {
  let protocolVersion;
  let lastBlockHeight;
  let lastBlockAppHash;
  let infoHandler;
  let updateSimplifiedMasternodeListMock;
  let lastCoreChainLockedHeight;
  let loggerMock;
  let blockExecutionContextMock;
  let blockExecutionContextRepositoryMock;
  let groveDBStoreMock;
  let createContextLoggerMock;

  beforeEach(function beforeEach() {
    lastBlockHeight = Long.fromInt(0);
    lastBlockAppHash = Buffer.alloc(0);
    protocolVersion = Long.fromInt(1);
    lastCoreChainLockedHeight = 0;

    updateSimplifiedMasternodeListMock = this.sinon.stub();

    loggerMock = new LoggerMock(this.sinon);

    blockExecutionContextMock = new BlockExecutionContextMock(this.sinon);
    blockExecutionContextMock.getHeight.returns(lastBlockHeight);
    blockExecutionContextMock.getCoreChainLockedHeight.returns(lastCoreChainLockedHeight);
    blockExecutionContextRepositoryMock = new BlockExecutionContextRepositoryMock(
      this.sinon,
    );
    groveDBStoreMock = new GroveDBStoreMock(this.sinon);

    blockExecutionContextRepositoryMock.fetch.resolves(blockExecutionContextMock);

    groveDBStoreMock.getRootHash.resolves(lastBlockAppHash);

    createContextLoggerMock = this.sinon.stub().returns(loggerMock);

    infoHandler = infoHandlerFactory(
      blockExecutionContextMock,
      blockExecutionContextRepositoryMock,
      protocolVersion,
      updateSimplifiedMasternodeListMock,
      loggerMock,
      groveDBStoreMock,
      createContextLoggerMock,
    );
  });

  it('should return respond with genesis heights and app hash on the first run', async () => {
    blockExecutionContextRepositoryMock.fetch.resolves(blockExecutionContextMock);
    blockExecutionContextMock.getHeight.returns(null);
    blockExecutionContextMock.isEmpty.returns(true);

    const response = await infoHandler();

    expect(response).to.be.an.instanceOf(ResponseInfo);

    expect(response).to.deep.include({
      version: packageJson.version,
      appVersion: protocolVersion,
      lastBlockHeight,
      lastBlockAppHash,
    });

    expect(blockExecutionContextRepositoryMock.fetch).to.be.calledOnce();
    expect(blockExecutionContextMock.populate).to.not.be.called();
    expect(blockExecutionContextMock.getHeight).to.not.be.called();
    expect(blockExecutionContextMock.getCoreChainLockedHeight).to.not.be.called();
    expect(updateSimplifiedMasternodeListMock).to.not.be.called();
    expect(groveDBStoreMock.getRootHash).to.be.calledOnce();
    expect(createContextLoggerMock).to.be.calledOnceWithExactly(
      loggerMock, {
        abciMethod: 'info',
      },
    );
  });

  it('should populate context and update SML on subsequent runs', async () => {
    const response = await infoHandler();

    expect(response).to.be.an.instanceOf(ResponseInfo);

    expect(ResponseInfo.toObject(response)).to.deep.equal({
      version: packageJson.version,
      appVersion: protocolVersion,
      lastBlockHeight,
      lastBlockAppHash,
    });

    expect(blockExecutionContextMock.getHeight).to.be.calledOnce();

    expect(updateSimplifiedMasternodeListMock).to.be.calledOnceWithExactly(
      lastCoreChainLockedHeight,
      {
        logger: loggerMock,
      },
    );
    expect(createContextLoggerMock).to.be.calledTwice();
    expect(createContextLoggerMock.getCall(0)).to.be.calledWithExactly(
      loggerMock, {
        abciMethod: 'info',
      },
    );
    expect(createContextLoggerMock.getCall(1)).to.be.calledWithExactly(
      loggerMock, {
        height: lastBlockHeight.toString(),
      },
    );
  });
});
