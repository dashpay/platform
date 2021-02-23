const _ = require('lodash');
const EventEmitter = require('events');
const { InjectionToPluginUnallowed } = require('../errors');
const { SAFE_FUNCTIONS, SAFE_PROPERTIES } = require('../CONSTANTS').INJECTION_LISTS;

const defaultOpts = {
  executeOnStart: false,
};

class StandardPlugin extends EventEmitter {
  constructor(opts = {}) {
    super();
    this.pluginType = _.has(opts, 'type') ? opts.type : 'Standard';
    this.name = _.has(opts, 'name') ? opts.name : 'UnnamedPlugin';
    this.dependencies = _.has(opts, 'dependencies') ? opts.dependencies : [];

    this.executeOnStart = _.has(opts, 'executeOnStart')
      ? opts.executeOnStart
      : defaultOpts.executeOnStart;

    // Apply other props
    Object.keys(opts).forEach((key) => {
      if (!this[key]) {
        this[key] = opts[key];
      }
    });
  }

  async startPlugin() {
    const self = this;

    try {
      if (this.executeOnStart === true && this.onStart) {
        await this.onStart();
      }
      const eventType = `PLUGIN/${this.name.toUpperCase()}/STARTED`;
      self.parentEvents.emit(eventType, { type: eventType, payload: null });
    } catch (e) {
      this.emit('error', e, {
        type: 'plugin',
        pluginType: 'plugin',
        pluginName: this.name,
      });
    }
  }

  inject(name, obj, allowSensitiveOperations = false) {
    const PLUGINS_NAME_LIST = [];
    if (SAFE_FUNCTIONS.includes(name) || SAFE_PROPERTIES.includes(name)) {
      this[name] = obj;
    } else if (PLUGINS_NAME_LIST.includes(name)) {
      this.emit('error', new Error('Inter-plugin support yet to come'), {
        type: 'plugin',
        pluginType: 'plugin',
        pluginName: this.name,
      });
    } else if (allowSensitiveOperations === true) {
      this[name] = obj;
    } else if (name === 'parentEvents') {
      // Called by injectPlugin to setup the parentEvents on/emit fn.
      // console.log(obj)
      // this.parentEvents = {on:obj.on, emit:obj.emit};
      this.parentEvents = obj;
    } else {
      this.emit('error', new InjectionToPluginUnallowed(name), {
        type: 'plugin',
        pluginType: 'plugin',
        pluginName: this.name,
      });
    }
    return true;
  }
}

module.exports = StandardPlugin;
