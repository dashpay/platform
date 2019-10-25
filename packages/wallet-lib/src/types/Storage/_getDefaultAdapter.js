const localForage = require('localforage');
const logger = require('../../logger');
const InMem = require('../../adapters/InMem');

module.exports = async function getDefaultAdapter() {
  const isBrowser = (typeof document !== 'undefined');
  // eslint-disable-next-line no-undef
  const isReactNative = (typeof navigator !== 'undefined' && navigator.product === 'ReactNative');
  const isNode = !isBrowser && !isReactNative;

  if (isNode) {
    logger.warn('NodeJS env - Specify an adapter, fallback on inMem storage only.');
    return InMem;
  }
  if (isReactNative) {
    logger.warn('React Native env - Specify an adapter, fallback on inMem storage only.');
    return InMem;
  } if (isBrowser) {
    return localForage;
  }
  throw new Error('Undetected platform - No default adapter.');
};
