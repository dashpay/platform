const _ = require('lodash');
const {
  InjectionErrorCannotInject,
  InjectionErrorCannotInjectUnknownDependency: InjectionErrorCannotInjectUnknownDep,
} = require('../../../errors');
const { is } = require('../../../utils');
const logger = require('../../../logger');
/**
 * Will try to inject a given plugin. If needed, it will construct the object first (new).
 * @param {Plugin} UnsafePlugin - Either a child object, or it's parent class to inject
 * @param {Boolean} [allowSensitiveOperations=false] - forcing injection discarding unsafeOp checks.
 * @param {Boolean} [awaitOnInjection=true] - When true, wait for onInjected resolve first
 * @return {Promise<*>} plugin - instance of the plugin
 */
module.exports = async function injectPlugin(
  UnsafePlugin,
  allowSensitiveOperations = false,
  awaitOnInjection = true,
) {
  // TODO : Only called internally, it might be worth to remove public access to it.
  // For now, it helps us on debugging
  const self = this;
  // eslint-disable-next-line no-async-promise-executor
  return new Promise(async (resolve, reject) => {
    try {
      const isInit = !(typeof UnsafePlugin === 'function');
      const plugin = (isInit) ? UnsafePlugin : new UnsafePlugin();
      plugin.on('error', (e, errContext) => self.emit('error', e, errContext));

      const pluginName = plugin.name.toLowerCase();
      logger.debug(`Account.injectPlugin(${pluginName}) - starting injection`);
      if (_.isEmpty(plugin)) reject(new InjectionErrorCannotInject(pluginName, 'Empty plugin'));

      // All plugins will require the event object
      const { pluginType } = plugin;

      const {
        on, emit,
        _conf, _maxListeners, _on, _events, _all, _newListener, _removeListener, listenerTree,
      } = self;

      plugin.inject('parentEvents', {
        on,
        emit,
        _conf,
        _maxListeners,
        wildcard: true,
        _on,
        _events,
        _all,
        _newListener,
        _removeListener,
        listenerTree,
      });

      // Check for dependencies
      const deps = plugin.dependencies || [];

      const injectedPlugins = Object.keys(this.plugins.standard).map((key) => key.toLowerCase());
      deps.forEach((dependencyName) => {
        if (_.has(self, dependencyName)) {
          plugin.inject(dependencyName, self[dependencyName], allowSensitiveOperations);
        } else if (typeof self[dependencyName] === 'function') {
          plugin.inject(dependencyName, self[dependencyName].bind(self), allowSensitiveOperations);
        } else {
          const loweredDependencyName = dependencyName.toLowerCase();
          if (injectedPlugins.includes(loweredDependencyName)) {
            plugin.inject(dependencyName, this.plugins.standard[loweredDependencyName], true);
          } else reject(new InjectionErrorCannotInjectUnknownDep(pluginName, dependencyName));
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
                announced: false,
              };
              self.plugins.watchers[pluginName] = watcher;

              // eslint-disable-next-line no-return-assign,no-param-reassign
              const startWatcher = (_watcher) => _watcher.started = true;
              // eslint-disable-next-line no-return-assign,no-param-reassign
              const setReadyWatch = (_watcher) => _watcher.ready = true;

              const onStartedEvent = () => startWatcher(watcher)
                  && logger.silly(`WORKER/${pluginName.toUpperCase()}/STARTED`);
              const onExecuteEvent = () => setReadyWatch(watcher)
                  && logger.silly(`WORKER/${pluginName.toUpperCase()}/EXECUTED`);

              self.on(`WORKER/${pluginName.toUpperCase()}/STARTED`, onStartedEvent);
              self.on(`WORKER/${pluginName.toUpperCase()}/EXECUTED`, onExecuteEvent);
            }
            await plugin.startWorker();
          }
          break;
        case 'Standard':
          if (plugin.executeOnStart === true) {
            if (plugin.firstExecutionRequired === true) {
              const watcher = {
                ready: false,
                started: false,
                announced: false,
              };
              self.plugins.watchers[pluginName] = watcher;
              // eslint-disable-next-line no-return-assign,no-param-reassign,max-len
              const startWatcher = (_watcher) => { _watcher.started = true; _watcher.ready = true; };

              const onStartedEvent = () => startWatcher(watcher);
              self.on(`PLUGIN/${pluginName.toUpperCase()}/STARTED`, onStartedEvent);
            }
          }
          self.plugins.standard[pluginName] = plugin;
          await plugin.startPlugin();
          break;
        default:
          throw new Error(`Unable to inject plugin: ${pluginType}`);
      }

      if (is.fn(plugin.onInjected)) {
        if (awaitOnInjection) await plugin.onInjected();
        else plugin.onInjected();
      }

      logger.debug(`Account.injectPlugin(${pluginName}) - successfully injected`);
      return resolve(plugin);
    } catch (e) {
      return reject(e);
    }
  });
};
