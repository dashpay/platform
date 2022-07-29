const DAPIClient = require('@dashevo/dapi-client');

const {
  BlockHeadersProvider,
} = DAPIClient;

const mockBlockHeadersProvider = (sinon) => {
  const blockHeadersProvider = new BlockHeadersProvider();

  blockHeadersProvider.setCoreMethods({
    getBlock: sinon.stub(),
  });

  return blockHeadersProvider;
};

module.exports = mockBlockHeadersProvider;
