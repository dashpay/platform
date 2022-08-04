const DAPIClient = require('@dashevo/dapi-client');

const {
  BlockHeadersProvider,
} = DAPIClient;

const mockBlockHeadersProvider = (sinon, historicalStreams, continuousStream) => {
  const numStreams = historicalStreams.length;

  const blockHeadersProvider = new BlockHeadersProvider({
    maxParallelStreams: numStreams,
  });

  let currentStream = 0;
  blockHeadersProvider.setCoreMethods({
    getBlock: sinon.stub(),
    subscribeToBlockHeadersWithChainLocks: ({ from, count }) => {
      if (count > 0) {
        const stream = historicalStreams[currentStream];
        currentStream += 1;
        return stream;
      }
      return continuousStream;
    },
  });

  return blockHeadersProvider;
};

module.exports = mockBlockHeadersProvider;
