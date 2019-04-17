const _ = require('lodash');
const { InjectionToPluginUnallowed } = require('../errors');
const { SAFE_FUNCTIONS, SAFE_PROPERTIES } = require('../CONSTANTS').INJECTION_LISTS;

class StandardPlugin {
  constructor(opts) {
    this.pluginType = _.has(opts, 'type') ? opts.type : 'Standard';
    this.name = _.has(opts, 'name') ? opts.name : 'UnnamedPlugin';
    this.dependencies = _.has(opts, 'dependencies') ? opts.dependencies : [];
    this.events = null;

    // Apply other props
    Object.keys(opts).forEach((key) => {
      if (!this[key]) {
        this[key] = opts[key];
      }
    });
  }

  inject(name, obj, allowSensitiveOperations = false) {
    const PLUGINS_NAME_LIST = [];
    if (SAFE_FUNCTIONS.includes(name) || SAFE_PROPERTIES.includes(name)) {
      this[name] = obj;
    } else if (PLUGINS_NAME_LIST.includes(name)) {
      throw new Error('Inter-plugin support yet to come');
    } else if (allowSensitiveOperations === true) {
      this[name] = obj;
    } else {
      throw new InjectionToPluginUnallowed(name);
    }
    return true;
  }
}
module.exports = StandardPlugin;
