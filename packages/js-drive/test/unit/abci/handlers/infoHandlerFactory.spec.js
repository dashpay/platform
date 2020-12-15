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

  beforeEach(function beforeEach() {
    lastBlockHeight = 1;
    lastBlockAppHash = Buffer.alloc(0);
    protocolVersion = Long.fromInt(0);

    const chainInfo = new ChainInfo(lastBlockHeight);

    rootTreeMock = new RootTreeMock(this.sinon);

    infoHandler = infoHandlerFactory(chainInfo, protocolVersion, rootTreeMock);
  });

  it('should return ResponseInfo', async () => {
    rootTreeMock.getRootHash.returns(lastBlockAppHash);

    const response = await infoHandler();

    expect(response).to.be.an.instanceOf(ResponseInfo);

    expect(response).to.deep.include({
      version: packageJson.version,
      appVersion: protocolVersion,
      lastBlockHeight,
      lastBlockAppHash,
    });
  });
});
