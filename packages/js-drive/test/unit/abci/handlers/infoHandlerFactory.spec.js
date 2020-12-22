const Long = require('long');

const {
  tendermint: {
    abci: {
      ResponseInfo,
    },
  },
} = require('@dashevo/abci/types');

const infoHandlerFactory = require('../../../../lib/abci/handlers/infoHandlerFactory');

const ChainInfo = require('../../../../lib/chainInfo/ChainInfo');

const RootTreeMock = require('../../../../lib/test/mock/RootTreeMock');

const packageJson = require('../../../../package');

describe('infoHandlerFactory', () => {
  let protocolVersion;
  let lastBlockHeight;
  let lastBlockAppHash;
  let infoHandler;
  let rootTreeMock;
  let updateSimplifiedMasternodeListMock;
  let lastCoreChainLockedHeight;
  let loggerMock;
  let chainInfo;
  let chainInfoRepositoryMock;
  let containerMock;

  beforeEach(function beforeEach() {
    lastBlockHeight = Long.fromInt(0);
    lastBlockAppHash = Buffer.alloc(0);
    protocolVersion = Long.fromInt(0);
    lastCoreChainLockedHeight = 0;

    chainInfoRepositoryMock = {
      store: this.sinon.stub(),
      fetch: this.sinon.stub(),
    };

    chainInfo = new ChainInfo(
      lastBlockHeight,
      lastCoreChainLockedHeight,
    );

    chainInfoRepositoryMock.fetch.resolves(chainInfo);

    rootTreeMock = new RootTreeMock(this.sinon);
    rootTreeMock.getRootHash.returns(lastBlockAppHash);

    updateSimplifiedMasternodeListMock = this.sinon.stub();

    loggerMock = {
      debug: this.sinon.stub(),
    };

    containerMock = {
      register: this.sinon.stub(),
    };

    infoHandler = infoHandlerFactory(
      chainInfoRepositoryMock,
      protocolVersion,
      rootTreeMock,
      updateSimplifiedMasternodeListMock,
      loggerMock,
      containerMock,
    );
  });

  it('should return empty info', async () => {
    const response = await infoHandler();

    expect(response).to.be.an.instanceOf(ResponseInfo);

    expect(response).to.deep.include({
      version: packageJson.version,
      appVersion: protocolVersion,
      lastBlockHeight,
      lastBlockAppHash,
    });

    expect(updateSimplifiedMasternodeListMock).to.not.be.called();
  });

  it('should update SML to latest core chain locked height and return stored info', async () => {
    lastBlockHeight = Long.fromInt(1);
    lastCoreChainLockedHeight = 2;

    chainInfo.setLastBlockHeight(lastBlockHeight);
    chainInfo.setLastCoreChainLockedHeight(lastCoreChainLockedHeight);

    const response = await infoHandler();

    expect(response).to.be.an.instanceOf(ResponseInfo);

    expect(response).to.deep.include({
      version: packageJson.version,
      appVersion: protocolVersion,
      lastBlockHeight,
      lastBlockAppHash,
    });

    expect(updateSimplifiedMasternodeListMock).to.be.calledOnceWithExactly(
      lastCoreChainLockedHeight,
    );
  });
});
