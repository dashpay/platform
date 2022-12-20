const Worker = require('../../plugins/Worker');

const sortPlugins = require('./_sortPlugins');

const preparePlugins = function preparePlugins(account, userUnsafePlugins) {
  return new Promise((resolve, reject) => {
    try {
      const sortedPlugins = sortPlugins(account, userUnsafePlugins);
      const injectedPlugins = [];
      const injectPlugins = async () => {
        // Inject plugins
        for (let i = 0; i < sortedPlugins.length; i += 1) {
          const [PluginClass, allowSensitiveOperation, awaitOnInjection] = sortedPlugins[i];
          // eslint-disable-next-line no-await-in-loop
          const plugin = await account.injectPlugin(
            PluginClass,
            allowSensitiveOperation,
            awaitOnInjection,
          );
          injectedPlugins.push(plugin);
        }
      };

      const executePlugins = async () => {
        // Execute plugins
        for (let i = 0; i < injectedPlugins.length; i += 1) {
          const plugin = injectedPlugins[i];
          if (plugin.executeOnStart && plugin instanceof Worker) {
            // eslint-disable-next-line no-await-in-loop
            await plugin.execWorker();
          }
        }
      };

      injectPlugins().then(executePlugins)
        .then(() => resolve(sortPlugins))
        .catch(reject);
    } catch (e) {
      reject(e);
    }
  });
};

module.exports = preparePlugins;
