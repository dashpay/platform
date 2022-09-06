const DAPIClient = require('@dashevo/dapi-client');

const {
  BlockHeadersProvider,
} = DAPIClient;

const mockBlockHeadersProvider = (sinon, historicalStreams, continuousStream, headersPerStream) => {
  const numStreams = historicalStreams.length;

  const blockHeadersProvider = new BlockHeadersProvider({
    maxParallelStreams: numStreams,
    targetBatchSize: headersPerStream,
  });

  let currentStream = 0;
  blockHeadersProvider.setCoreMethods({
    getBlock: sinon.stub(),
    subscribeToBlockHeadersWithChainLocks: ({ count }) => {
      if (count > 0) {
        const stream = historicalStreams[currentStream];

        if (currentStream === historicalStreams.length - 1) {
          currentStream = 0;
        } else {
          currentStream += 1;
        }
        return stream;
      }
      return continuousStream;
    },
  });

  return blockHeadersProvider;
};

module.exports = mockBlockHeadersProvider;
