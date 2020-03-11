const logger = require('../../logger');
const InMem = require('../../adapters/InMem');

module.exports = async function getDefaultAdapter() {
  const isBrowser = (typeof document !== 'undefined');
  // eslint-disable-next-line no-undef
  const isReactNative = (typeof navigator !== 'undefined' && navigator.product === 'ReactNative');
  const isNode = !isBrowser && !isReactNative;

  if (isNode) {
    logger.warn('Running on a NodeJS env without any specified adapter. Data will not persist.');
    return InMem;
  }
  if (isReactNative) {
    logger.warn('Running on a React Native env without any specified adapter. Data will not persist.');
    return InMem;
  } if (isBrowser) {
    return InMem;
  }
  throw new Error('Undetected platform - No default adapter to persist data to.');
};
