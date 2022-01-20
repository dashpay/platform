const networks = require('@dashevo/dashcore-lib/lib/networks');
const DAPIClientError = require('../errors/DAPIClientError');
const BlockHeadersProvider = require('./BlockHeadersProvider');

/**
 * @typedef {createBlockHeadersProviderFromOptions}
 * @param {DAPIClientOptions} options
 * @param {CoreMethodsFacade} coreMethods
 * @returns {BlockHeadersProvider}
 */
function createBlockHeadersProviderFromOptions(options, coreMethods) {
  let blockHeadersProvider;
  if (options.blockHeadersProvider) {
    if (options.blockHeadersProviderOptions) {
      throw new DAPIClientError("Can't use 'blockHeadersProviderOptions' with 'blockHeadersProvider' option");
    }

    blockHeadersProvider = options.blockHeadersProvider;
  }

  if (options.blockHeadersProviderOptions) {
    const blockHeadersProviderOptions = { ...options, ...BlockHeadersProvider.defaultOptions };

    const {
      network,
      autoStart,
      maxParallelStreams,
      targetBatchSize,
      fromBlockHeight,
    } = blockHeadersProviderOptions;

    if (network && !networks.get(network)) {
      throw new DAPIClientError(`Invalid network '${options.network}'`);
    }

    if (typeof autoStart !== 'boolean') {
      throw new DAPIClientError('\'autoStart\' option must have boolean type');
    }

    if (typeof maxParallelStreams !== 'number') {
      throw new DAPIClientError('\'maxParallelStreams\' is not a number');
    }

    if (maxParallelStreams < 1) {
      throw new DAPIClientError('\'maxParallelStreams\' can not be less than 1');
    }

    if (typeof targetBatchSize !== 'number') {
      throw new DAPIClientError('\'targetBatchSize\' is not a number');
    }

    if (targetBatchSize < 1) {
      throw new DAPIClientError('\'targetBatchSize\' can not be less than 1');
    }

    if (typeof fromBlockHeight !== 'number') {
      throw new DAPIClientError('\'fromBlockHeight\' is not a number');
    }

    if (fromBlockHeight < 1) {
      throw new DAPIClientError('\'fromBlockHeight\' can not be less than');
    }

    blockHeadersProvider = new BlockHeadersProvider(
      blockHeadersProviderOptions,
    );
  }

  if (!blockHeadersProvider) {
    blockHeadersProvider = new BlockHeadersProvider();
  }

  blockHeadersProvider.setCoreMethods(coreMethods);

  return blockHeadersProvider;
}

module.exports = createBlockHeadersProviderFromOptions;
