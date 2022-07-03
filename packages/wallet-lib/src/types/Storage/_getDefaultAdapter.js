const logger = require('../../logger');
const InMem = require('../../adapters/InMem');

module.exports = async function getDefaultAdapter() {
  const isBrowser = (typeof document !== 'undefined');
  // eslint-disable-next-line no-undef
  const isReactNative = (typeof navigator !== 'undefined' && navigator.product === 'ReactNative');
  const isNode = !isBrowser && !isReactNative;

  function getWarn(env) {
    return [
      `Running on ${env} without 'dapiOpts.wallet.adapter' for address storage. Data will not persist.`,
      `See <https://github.com/dashevo/platform/blob/master/packages/wallet-lib/src/adapters/InMem.js>`,
      `and <https://github.com/coolaj86/platform-readme-tutorials/blob/better-storage-example/create-wallet.js>`,
    ].join('\n');
  }
  
  if (isNode) {
    logger.warn(getWarn("a NodeJS env"));
    return InMem;
  }
  if (isReactNative) {
    logger.warn(getWarn("a React Native env"));
    return InMem;
  } if (isBrowser) {
    return InMem;
  }
  throw new Error(getWarn("an unknown platform"));
};
