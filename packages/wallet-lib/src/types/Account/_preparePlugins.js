const sortPlugins = require('./_sortPlugins');
const logger = require('../../logger');

const preparePlugins = function preparePlugins(account, userUnsafePlugins) {
  function reducer(accumulatorPromise, [plugin, allowSensitiveOperation, awaitOnInjection]) {
    return accumulatorPromise
      .then(async () => {
        try {
          await account.injectPlugin(
            plugin,
            allowSensitiveOperation,
            awaitOnInjection,
          );
        } catch (e) {
          logger.error('Error injecting plugin', e);
          this.emit('error', e, {
            type: 'plugin',
            pluginType: 'plugin',
            pluginName: plugin.name,
          });
        }
      });
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
