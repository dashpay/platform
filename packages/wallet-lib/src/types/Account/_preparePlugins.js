const sortPlugins = require('./_sortPlugins');

const preparePlugins = function preparePlugins(account, userUnsafePlugins) {
  function reducer(accumulatorPromise, [plugin, allowSensitiveOperation, awaitOnInjection]) {
    return accumulatorPromise
      .then(() => account.injectPlugin(plugin, allowSensitiveOperation, awaitOnInjection));
  }

  return new Promise((resolve, reject) => {
    try {
      const sortedPlugins = sortPlugins(account, userUnsafePlugins);
      // It is important that all plugin got successfully injected in a sequential maneer
      sortedPlugins.reduce(reducer, Promise.resolve()).then(() => resolve(sortedPlugins));

      resolve(sortedPlugins);
    } catch (e) {
      reject(e);
    }
  });
};

module.exports = preparePlugins;
