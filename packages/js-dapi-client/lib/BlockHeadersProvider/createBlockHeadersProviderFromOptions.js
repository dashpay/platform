const DAPIClientError = require('../errors/DAPIClientError');
const BlockHeadersProvider = require('./BlockHeadersProvider');
const ReconnectableStream = require('../transport/ReconnectableStream');

const validateNumber = (value, name, min = NaN, max = NaN) => {
  if (typeof value !== 'number') {
    throw new DAPIClientError(`'${name}' is not a number`);
  }

  if (!Number.isNaN(min) && value < min) {
    throw new DAPIClientError(`'${name}' can not be less than ${min}`);
  }

  if (!Number.isNaN(max) && value > max) {
    throw new DAPIClientError(`'${name}' can not be more than ${max}`);
  }
};

/**
 * @typedef {createBlockHeadersProviderFromOptions}
 * @param {DAPIClientOptions} options
 * @param logger
 * @param {CoreMethodsFacade} coreMethods
 * @returns {BlockHeadersProvider}
 */
function createBlockHeadersProviderFromOptions(options, coreMethods, logger) {
  let blockHeadersProvider;
  if (options.blockHeadersProvider) {
    if (options.blockHeadersProviderOptions) {
      throw new DAPIClientError('Can\'t use \'blockHeadersProviderOptions\' with \'blockHeadersProvider\' option');
    }

    blockHeadersProvider = options.blockHeadersProvider;
  }

  const createContinuousSyncStream = (fromBlockHeight) => ReconnectableStream
    .create(
      coreMethods.subscribeToBlockHeadersWithChainLocks,
      {
        maxRetriesOnError: -1,
        logger,
      },
    )({
      fromBlockHeight,
    });

  const createHistoricalSyncStream = (fromBlockHeight, count) => {
    const { subscribeToBlockHeadersWithChainLocks } = coreMethods;
    return subscribeToBlockHeadersWithChainLocks({
      fromBlockHeight,
      count,
    });
  };

  if (options.blockHeadersProviderOptions) {
    const { network } = options;

    const blockHeadersProviderOptions = {
      ...BlockHeadersProvider.defaultOptions,
      ...options.blockHeadersProviderOptions,
      network,
    };

    const {
      maxParallelStreams,
      targetBatchSize,
      fromBlockHeight,
      maxRetries,
    } = blockHeadersProviderOptions;

    validateNumber(maxParallelStreams, 'maxParallelStreams', 1);
    validateNumber(targetBatchSize, 'targetBatchSize', 1);
    validateNumber(fromBlockHeight, 'fromBlockHeight', 1);
    validateNumber(maxRetries, 'maxRetries', 0, 100);

    blockHeadersProvider = new BlockHeadersProvider(
      blockHeadersProviderOptions,
      createHistoricalSyncStream,
      createContinuousSyncStream,
    );
  }

  if (!blockHeadersProvider) {
    blockHeadersProvider = new BlockHeadersProvider(
      {},
      createContinuousSyncStream,
      createHistoricalSyncStream,
    );
  }

  return blockHeadersProvider;
}

module.exports = createBlockHeadersProviderFromOptions;
