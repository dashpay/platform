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
const BlockExecutionContextStackMock = require('../../../../lib/test/mock/BlockExecutionContextStackMock');
const BlockExecutionContextStackRepositoryMock = require('../../../../lib/test/mock/BlockExecutionContextStackRepositoryMock');

describe('infoHandlerFactory', () => {
  let protocolVersion;
  let lastBlockHeight;
  let lastBlockAppHash;
  let infoHandler;
  let updateSimplifiedMasternodeListMock;
  let lastCoreChainLockedHeight;
  let loggerMock;
  let blockExecutionContextMock;
  let blockExecutionContextStackMock;
  let blockExecutionContextStackRepositoryMock;
  let groveDBStoreMock;

  beforeEach(function beforeEach() {
    lastBlockHeight = Long.fromInt(0);
    lastBlockAppHash = Buffer.alloc(0);
    protocolVersion = Long.fromInt(1);
    lastCoreChainLockedHeight = 0;

    updateSimplifiedMasternodeListMock = this.sinon.stub();

    loggerMock = new LoggerMock(this.sinon);

    blockExecutionContextMock = new BlockExecutionContextMock(this.sinon);
    blockExecutionContextStackMock = new BlockExecutionContextStackMock(this.sinon);
    blockExecutionContextStackRepositoryMock = new BlockExecutionContextStackRepositoryMock(
      this.sinon,
    );
    groveDBStoreMock = new GroveDBStoreMock(this.sinon);

    blockExecutionContextStackRepositoryMock.fetch.resolves({
      getContexts: this.sinon.stub(),
    });

    groveDBStoreMock.getRootHash.resolves(lastBlockAppHash);

    infoHandler = infoHandlerFactory(
      blockExecutionContextStackMock,
      blockExecutionContextStackRepositoryMock,
      blockExecutionContextMock,
      protocolVersion,
      updateSimplifiedMasternodeListMock,
      loggerMock,
      groveDBStoreMock,
    );
  });

  it('should return respond with genesis heights and app hash on the first run', async () => {
    const response = await infoHandler();

    expect(response).to.be.an.instanceOf(ResponseInfo);

    expect(response).to.deep.include({
      version: packageJson.version,
      appVersion: protocolVersion,
      lastBlockHeight,
      lastBlockAppHash,
    });

    expect(blockExecutionContextStackRepositoryMock.fetch).to.be.calledOnce();
    expect(blockExecutionContextStackMock.getFirst).to.be.calledOnce();
    expect(blockExecutionContextMock.populate).to.not.be.called();
    expect(blockExecutionContextMock.getHeader).to.not.be.called();
    expect(updateSimplifiedMasternodeListMock).to.not.be.called();
    expect(groveDBStoreMock.getRootHash).to.be.calledOnce();
  });

  it('should populate context, initialize Credits Distribution Pool and update SML on subsequent runs', async () => {
    blockExecutionContextStackMock.getFirst.returns(blockExecutionContextMock);

    lastBlockHeight = Long.fromInt(1);
    lastCoreChainLockedHeight = 2;

    blockExecutionContextMock.getHeader.returns({
      height: lastBlockHeight,
      coreChainLockedHeight: lastCoreChainLockedHeight,
    });

    const response = await infoHandler();

    expect(response).to.be.an.instanceOf(ResponseInfo);

    expect(ResponseInfo.toObject(response)).to.deep.equal({
      version: packageJson.version,
      appVersion: protocolVersion,
      lastBlockHeight,
      lastBlockAppHash,
    });

    expect(blockExecutionContextMock.getHeader).to.be.calledOnce();

    expect(updateSimplifiedMasternodeListMock).to.be.calledOnceWithExactly(
      lastCoreChainLockedHeight,
      {
        logger: loggerMock,
      },
    );
  });
});
