const DAPIClient = require('@dashevo/dapi-client');

const {
  BlockHeadersProvider,
} = DAPIClient;

const mockBlockHeadersProvider = (sinon, historicalStreams, continuousStream, headersPerStream) => {
  const numStreams = historicalStreams.length;

  let currentStream = 0;
  const subscribeToBlockHeadersWithChainLocks = ({ count }) => {
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
  };

  return new BlockHeadersProvider(
    {
      maxParallelStreams: numStreams,
      targetBatchSize: headersPerStream,
    },
    (fromBlockHeight, count) => subscribeToBlockHeadersWithChainLocks({
      fromBlockHeight,
      count,
    }),
    () => subscribeToBlockHeadersWithChainLocks({
      count: 0,
    }),
  );
};

module.exports = mockBlockHeadersProvider;
