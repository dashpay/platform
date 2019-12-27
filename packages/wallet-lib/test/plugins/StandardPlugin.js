const { expect } = require('chai');
const { EventEmitter } = require('events');
const StandardPlugin = require('../../src/plugins/StandardPlugin');

describe('Plugins - StandardPlugin', function suite() {
  this.timeout(60000);
  let plugin;
  let didSomething = 0;
  it('should initiate', async () => {
    plugin = new StandardPlugin();
    plugin.provideSomething = () => {
      didSomething += 1;
      return true;
    };
    expect(plugin).to.not.equal(null);
    expect(plugin.pluginType).to.equal('Standard');
    expect(plugin.name).to.equal('UnnamedPlugin');
    expect(plugin.dependencies).to.deep.equal([]);
    expect(plugin.events).to.equal(null);
  });
  it('should inject an event emitter', () => {
    const emitter = new EventEmitter();
    plugin.inject('events', emitter);
    expect(plugin.events).to.deep.equal(emitter);
  });
  it('should provide methods', () => {
    expect(plugin.provideSomething()).to.equal(true);
    expect(didSomething).to.deep.equal(1);
  });
});
