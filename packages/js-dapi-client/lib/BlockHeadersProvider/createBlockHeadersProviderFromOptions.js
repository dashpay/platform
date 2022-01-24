const networks = require('@dashevo/dashcore-lib/lib/networks');
const DAPIClientError = require('../errors/DAPIClientError');
const BlockHeadersProvider = require('./BlockHeadersProvider');

const validateNumber = (value, name, min = NaN, max = NaN) => {
  if (typeof value !== 'number') {
    throw new DAPIClientError(`'${name}' is not a number`);
  }

  if (!Number.isNaN(min) && value < min) {
    throw new DAPIClientError(`'${name}' can not be less than ${min}`);
  }

  if (!Number.isNaN(max) && value > min) {
    throw new DAPIClientError(`'${name}' can not be more than ${max}`);
  }
};

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
    const blockHeadersProviderOptions = {
      ...BlockHeadersProvider.defaultOptions,
      ...options.blockHeadersProviderOptions,
    };

    const {
      network,
      autoStart,
      maxParallelStreams,
      targetBatchSize,
      fromBlockHeight,
      maxRetries,
    } = blockHeadersProviderOptions;

    if (network && !networks.get(network)) {
      throw new DAPIClientError(`Invalid network '${options.network}'`);
    }

    if (typeof autoStart !== 'boolean') {
      throw new DAPIClientError('\'autoStart\' option must have boolean type');
    }

    validateNumber(maxParallelStreams, 'maxParallelStreams', 1);
    validateNumber(targetBatchSize, 'targetBatchSize', 1);
    validateNumber(fromBlockHeight, 'fromBlockHeight', 1);
    validateNumber(maxRetries, 'maxRetries', 0);

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
