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

  beforeEach(function beforeEach() {
    lastBlockHeight = Long.fromInt(0);
    lastBlockAppHash = Buffer.alloc(0);
    protocolVersion = Long.fromInt(1);
    lastCoreChainLockedHeight = 0;

    updateSimplifiedMasternodeListMock = this.sinon.stub();

    loggerMock = new LoggerMock(this.sinon);

    blockExecutionContextMock = new BlockExecutionContextMock(this.sinon);
    blockExecutionContextMock.getPreviousHeight.returns(lastBlockHeight);
    blockExecutionContextMock.getPreviousCoreChainLockedHeight.returns(lastCoreChainLockedHeight);
    blockExecutionContextRepositoryMock = new BlockExecutionContextRepositoryMock(
      this.sinon,
    );
    groveDBStoreMock = new GroveDBStoreMock(this.sinon);

    blockExecutionContextRepositoryMock.fetch.resolves(blockExecutionContextMock);

    groveDBStoreMock.getRootHash.resolves(lastBlockAppHash);

    infoHandler = infoHandlerFactory(
      blockExecutionContextMock,
      blockExecutionContextRepositoryMock,
      protocolVersion,
      updateSimplifiedMasternodeListMock,
      loggerMock,
      groveDBStoreMock,
    );
  });

  it('should return respond with genesis heights and app hash on the first run', async () => {
    blockExecutionContextRepositoryMock.fetch.resolves(null);
    blockExecutionContextMock.getPreviousHeight.returns(null);

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
  });

  it('should populate context and update SML on subsequent runs', async () => {
    blockExecutionContextMock.getHeight.returns(lastBlockHeight);
    blockExecutionContextMock.getCoreChainLockedHeight.returns(lastCoreChainLockedHeight);

    const response = await infoHandler();

    expect(response).to.be.an.instanceOf(ResponseInfo);

    expect(ResponseInfo.toObject(response)).to.deep.equal({
      version: packageJson.version,
      appVersion: protocolVersion,
      lastBlockHeight,
      lastBlockAppHash,
    });

    expect(blockExecutionContextMock.getPreviousHeight).to.be.calledThrice();

    expect(updateSimplifiedMasternodeListMock).to.be.calledOnceWithExactly(
      lastCoreChainLockedHeight,
      {
        logger: loggerMock,
      },
    );
  });
});
