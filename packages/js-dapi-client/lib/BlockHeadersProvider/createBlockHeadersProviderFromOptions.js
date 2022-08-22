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
    const { network } = options;

    // TODO: get rid of param reassing and pass network directly to BlockHeadersProvider
    // eslint-disable-next-line
    options.blockHeadersProviderOptions.network = network;

    const blockHeadersProviderOptions = {
      ...BlockHeadersProvider.defaultOptions,
      ...options.blockHeadersProviderOptions,
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
