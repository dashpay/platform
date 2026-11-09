const DAPIClient = require('@dashevo/dapi-client');

const {
  BlockHeadersProvider,
} = DAPIClient;

const mockBlockHeadersProvider = (
  sinon,
  historicalStreams,
  continuousStream,
  headersPerStream,
  network = 'testnet',
) => {
  const numStreams = historicalStreams.length;

  let currentStream = 0;
  const subscribeToBlockHeadersWithChainLocks = ({ fromBlockHeight, count }) => {
    if (count > 0) {
      const stream = historicalStreams[currentStream];
      // The range the reader asked this stream for, and how much was sent
      stream.range = { fromBlockHeight, count, sent: 0 };

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
      network,
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
