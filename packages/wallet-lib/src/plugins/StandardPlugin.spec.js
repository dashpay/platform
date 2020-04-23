const { expect } = require('chai');
const { EventEmitter } = require('events');
const StandardPluginSpec = require('./StandardPlugin');

describe('Plugins - StandardPlugin', function suite() {
  this.timeout(60000);
  let plugin;
  let didSomething = 0;
  it('should initiate', async () => {
    plugin = new StandardPluginSpec();
    plugin.provideSomething = () => {
      didSomething += 1;
      return true;
    };
    expect(plugin).to.not.equal(null);
    expect(plugin.pluginType).to.equal('Standard');
    expect(plugin.name).to.equal('UnnamedPlugin');
    expect(plugin.dependencies).to.deep.equal([]);
  });
  it('should inject an event emitter', () => {
    const emitter = new EventEmitter();
    plugin.inject('parentEvents', { on: emitter.on, emit: emitter.emit });
    expect(plugin.parentEvents.on).to.deep.equal(emitter.on);
    expect(plugin.parentEvents.emit).to.deep.equal(emitter.emit);
  });
  it('should provide methods', () => {
    expect(plugin.provideSomething()).to.equal(true);
    expect(didSomething).to.deep.equal(1);
  });
});
