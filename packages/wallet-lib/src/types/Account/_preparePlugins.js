const sortPlugins = require('./_sortPlugins');
const logger = require('../../logger');

const preparePlugins = function preparePlugins(account, userUnsafePlugins) {
  const self = this;

  function reducer(accumulatorPromise, plugins) {
    const injectPlugin = async ([plugin, allowSensitiveOperation, awaitOnInjection]) => {
      try {
        await account.injectPlugin(plugin, allowSensitiveOperation, awaitOnInjection);
      } catch (e) {
        logger.error('Error injecting plugin', e);
        self.emit('error', e, {
          type: 'plugin',
          pluginType: 'plugin',
          pluginName: plugin.name,
        });
      }
    };

    return accumulatorPromise
      .then(async () => {
        // For parallel executed plugins
        if (Array.isArray(plugins) && Array.isArray(plugins[0])) {
          return Promise.all(plugins.map((pluginConfig) => injectPlugin(pluginConfig)));
        }
        return injectPlugin(plugins);
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
