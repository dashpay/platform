const _ = require('lodash');
const {
  InjectionErrorCannotInject,
  InjectionErrorCannotInjectUnknownDependency,
} = require('../errors/index');
const { is } = require('../utils');
/**
 * Will try to inject a given plugin. If needed, it will construct the object first (new).
 * @param UnsafePlugin - Either a child object, or it's parent class to inject
 * @param allowSensitiveOperations (false) - When true, force injection discarding unsafeOp checks.
 * @param awaitOnInjection (true) - When true, wait for onInjected resolve first
 * @return {Promise<*>}
 */
module.exports = async function injectPlugin(
  UnsafePlugin,
  allowSensitiveOperations = false,
  awaitOnInjection = true,
) {
  // TODO : Only called internally, it might be worth to remove public access to it.
  // For now, it helps us on debugging
  const self = this;
  return new Promise(async (res, rej) => {
    const isInit = !(typeof UnsafePlugin === 'function');
    const plugin = (isInit) ? UnsafePlugin : new UnsafePlugin();

    const pluginName = plugin.name.toLowerCase();

    if (_.isEmpty(plugin)) rej(new InjectionErrorCannotInject(pluginName, 'Empty plugin'));

    // All plugins will require the event object
    const { pluginType } = plugin;

    plugin.inject('events', self.events);

    // Check for dependencies
    const deps = plugin.dependencies || [];

    const injectedPlugins = Object.keys(this.plugins.standard).map(key => key.toLowerCase());
    const injectedDaps = Object.keys(this.plugins.daps).map(key => key.toLowerCase());
    deps.forEach((dependencyName) => {
      if (_.has(self, dependencyName)) {
        plugin.inject(dependencyName, self[dependencyName], allowSensitiveOperations);
      } else if (typeof self[dependencyName] === 'function') {
        plugin.inject(dependencyName, self[dependencyName].bind(self), allowSensitiveOperations);
      } else {
        const loweredDependencyName = dependencyName.toLowerCase();
        if (injectedPlugins.includes(loweredDependencyName)) {
          plugin.inject(dependencyName, this.plugins.standard[loweredDependencyName], true);
        } else if (injectedDaps.includes(loweredDependencyName)) {
          plugin.inject(dependencyName, this.plugins.daps[loweredDependencyName], true);
        } else rej(new InjectionErrorCannotInjectUnknownDependency(pluginName, dependencyName));
      }
    });
    switch (pluginType) {
      case 'Worker':
        self.plugins.workers[pluginName] = plugin;
        if (plugin.executeOnStart === true) {
          if (plugin.firstExecutionRequired === true) {
            const watcher = {
              ready: false,
              started: false,
            };
            self.plugins.watchers[pluginName] = watcher;

            // eslint-disable-next-line no-return-assign,no-param-reassign
            const startWatcher = _watcher => _watcher.started = true;
            // eslint-disable-next-line no-return-assign,no-param-reassign
            const setReadyWatch = _watcher => _watcher.ready = true;

            const onStartedEvent = () => startWatcher(watcher);
            const onExecuteEvent = () => setReadyWatch(watcher);

            self.events.on(`WORKER/${pluginName.toUpperCase()}/STARTED`, onStartedEvent);
            self.events.on(`WORKER/${pluginName.toUpperCase()}/EXECUTED`, onExecuteEvent);
          }
          await plugin.startWorker();
        }
        break;
      case 'DAP':
        self.plugins.daps[pluginName] = plugin;
        break;
      case 'StandardPlugin':
      default:
        self.plugins.standard[pluginName] = plugin;
        break;
    }


    if (is.fn(plugin.onInjected)) {
      if (awaitOnInjection) await plugin.onInjected();
      else plugin.onInjected();
    }


    return res(plugin);
  });
};
