const _ = require('lodash');
const EventEmitter = require('events');

const defaultOpts = {
  executeOnStart: false,
};

class StandardPlugin extends EventEmitter {
  constructor(opts = {}) {
    super();
    this.pluginType = _.has(opts, 'type') ? opts.type : 'Standard';
    this.name = _.has(opts, 'name') ? opts.name : 'UnnamedPlugin';
    this.dependencies = _.has(opts, 'dependencies') ? opts.dependencies : [];
    this.injectionOrder = _.has(opts, 'injectionOrder') ? opts.injectionOrder : { before: [], after: [] };
    this.awaitOnInjection = _.has(opts, 'awaitOnInjection') ? opts.awaitOnInjection : false;
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

  inject(name, obj) {
    if (name === 'parentEvents') {
      // Called by injectPlugin to setup the parentEvents on/emit fn.
      this.parentEvents = obj;
    } else {
      this[name] = obj;
    }
    return true;
  }
}

module.exports = StandardPlugin;
