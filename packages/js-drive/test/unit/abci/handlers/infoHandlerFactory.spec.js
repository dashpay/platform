const {
  abci: {
    ResponseInfo,
  },
} = require('abci/types');

const infoHandlerFactory = require('../../../../lib/abci/handlers/infoHandlerFactory');

const BlockchainState = require('../../../../lib/blockchainState/BlockchainState');

const packageJson = require('../../../../package');

describe('infoHandlerFactory', () => {
  let lastBlockHeight;
  let lastBlockAppHash;
  let infoHandler;

  beforeEach(() => {
    lastBlockHeight = 1;
    lastBlockAppHash = Buffer.alloc(0);

    const blockchainState = new BlockchainState(lastBlockHeight, lastBlockAppHash);

    infoHandler = infoHandlerFactory(blockchainState);
  });

  it('should return ResponseInfo', async () => {
    const response = await infoHandler();

    expect(response).to.be.an.instanceOf(ResponseInfo);

    expect(response).to.deep.include({
      version: packageJson.version,
      lastBlockHeight,
      lastBlockAppHash,
    });
  });
});
