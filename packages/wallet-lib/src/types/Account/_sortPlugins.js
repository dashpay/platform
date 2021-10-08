const { each, findIndex } = require('lodash');
const TransactionSyncStreamWorker = require('../../plugins/Workers/TransactionSyncStreamWorker/TransactionSyncStreamWorker');
const ChainPlugin = require('../../plugins/Plugins/ChainPlugin');
const IdentitySyncWorker = require('../../plugins/Workers/IdentitySyncWorker');
const { WALLET_TYPES } = require('../../CONSTANTS');

const initPlugin = (UnsafePlugin) => {
  const isInit = !(typeof UnsafePlugin === 'function');
  return (isInit) ? UnsafePlugin : new UnsafePlugin();
};

/**
 * Sort user defined plugins using the injectionOrder properties before or after when specified.
 *
 * Except if specified using before property, all system plugins (TxSyncStream, IdentitySync...)
 * will be sorted on top.
 *
 * @param defaultSortedPlugins
 * @param userUnsafePlugins
 * @returns {*[]}
 */
const sortUserPlugins = (defaultSortedPlugins, userUnsafePlugins, allowSensitiveOperations) => {
  const sortedPlugins = [];
  const initializedSortedPlugins = [];

  // We start by ensuring all default plugins get loaded and initialized on top
  defaultSortedPlugins.forEach((defaultPluginParams) => {
    sortedPlugins.push(defaultPluginParams);

    // We also need to initialize them so we actually as we gonna need to read some properties.
    const plugin = initPlugin(defaultPluginParams[0]);
    initializedSortedPlugins.push(plugin);
  });

  // Iterate accross all user defined plugins
  each(userUnsafePlugins, (UnsafePlugin) => {
    const plugin = initPlugin(UnsafePlugin);

    const {
      awaitOnInjection,
      injectionOrder: {
        before: injectBefore,
        after: injectAfter,
      },
    } = plugin;

    const hasAfterDependencies = !!(injectAfter && injectAfter.length);
    const hasBeforeDependencies = !!(injectBefore && injectBefore.length);
    const hasPluginDependencies = (hasAfterDependencies || hasBeforeDependencies);

    let injectionIndex = initializedSortedPlugins.length;

    if (hasPluginDependencies) {
      let injectionBeforeIndex = -1;
      let injectionAfterIndex = -1;

      if (hasBeforeDependencies) {
        each(injectBefore, (pluginDependencyName) => {
          const beforePluginIndex = findIndex(initializedSortedPlugins, ['name', pluginDependencyName]);
          // TODO: we could have an handling that would postpone trying to insert the dependencies
          // ensuring the case where we try to rely and sort based on user specified dependencies
          // For now, require user to sort them when specifying the plugins.
          if (beforePluginIndex === -1) throw new Error(`Dependency ${pluginDependencyName} not found`);
          if (injectionBeforeIndex === -1 || injectionIndex > beforePluginIndex) {
            injectionBeforeIndex = (injectionBeforeIndex === -1 || injectBefore > beforePluginIndex)
              ? beforePluginIndex
              : injectionBeforeIndex;
          }
        });
      }

      if (hasAfterDependencies) {
        each(injectAfter, (pluginDependencyName) => {
          const afterPluginIndex = findIndex(initializedSortedPlugins, ['name', pluginDependencyName]);
          if (afterPluginIndex === -1) throw new Error(`Dependency ${pluginDependencyName} not found`);
          if (injectionAfterIndex === -1 || injectionAfterIndex < afterPluginIndex + 1) {
            injectionAfterIndex = afterPluginIndex + 1;
          }
        });
      }

      if (
        injectionBeforeIndex !== -1
          && injectionAfterIndex !== -1
          && injectionAfterIndex > injectionBeforeIndex
      ) {
        throw new Error(`Conflicting dependency order for ${plugin.name}`);
      }

      if (
        injectionBeforeIndex !== -1
          || injectionAfterIndex !== -1
      ) {
        injectionIndex = (injectionBeforeIndex !== -1)
          ? injectionBeforeIndex
          : injectionAfterIndex;
      }
    }

    // We insert both initialized and uninitialized plugins as we gonna need to read property.
    initializedSortedPlugins.splice(
      injectionIndex,
      0,
      plugin,
    );
    sortedPlugins.splice(
      injectionIndex,
      0,
      [UnsafePlugin, allowSensitiveOperations, awaitOnInjection],
    );
  });
  initializedSortedPlugins.forEach((initializedSortedPlugin, i) => {
    delete initializedSortedPlugins[i];
  });
  return sortedPlugins;
};

/**
 * Sort plugins defined by users based on the before and after properties
 * @param account
 * @param userUnsafePlugins
 * @returns {*[]}
 */
const sortPlugins = (account, userUnsafePlugins) => {
  const plugins = [];

  // eslint-disable-next-line no-async-promise-executor
  if (account.injectDefaultPlugins) {
    if (!account.offlineMode) {
      plugins.push([ChainPlugin, true, true]);
      plugins.push([TransactionSyncStreamWorker, true, true]);

      if (account.walletType === WALLET_TYPES.HDWALLET) {
        plugins.push([IdentitySyncWorker, true, true]);
      }
    }
  }
  return sortUserPlugins(plugins, userUnsafePlugins, account.allowSensitiveOperations);
};
module.exports = sortPlugins;
