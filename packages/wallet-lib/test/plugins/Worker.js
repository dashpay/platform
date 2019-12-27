const { expect } = require('chai');
const { EventEmitter } = require('events');
const Worker = require('../../src/plugins/Worker');

describe('Plugins - Worker', function suite() {
  this.timeout(60000);
  let worker;
  it('should initiate', async () => {
    worker = new Worker();

    expect(worker).to.not.equal(null);
    expect(worker.pluginType).to.equal('Worker');
    expect(worker.name).to.equal('UnnamedPlugin');
    expect(worker.dependencies).to.deep.equal([]);
    expect(worker.events).to.equal(null);
    expect(worker.workerIntervalTime).to.equal(10000);
    expect(worker.executeOnStart).to.equal(false);
    expect(worker.firstExecutionRequired).to.equal(false);
    expect(worker.workerMaxPass).to.equal(null);
    expect(worker.worker).to.equal(null);
    expect(worker.workerPass).to.equal(0);
    expect(worker.isWorkerRunning).to.equal(false);
  });
  it('should inject an event emitter', () => {
    const emitter = new EventEmitter();
    worker.inject('events', emitter);
    expect(worker.events).to.deep.equal(emitter);
  });
  it('should start and stop', (done) => {
    let didSomething = 0;
    worker.workerIntervalTime = 200;
    worker.execute = () => {
      didSomething += 1;
    };

    worker.startWorker();
    setTimeout(() => {
      expect(worker.workerPass).to.equal(4);
      expect(didSomething).to.equal(4);
      worker.stopWorker();
      setTimeout(() => {
        expect(worker.workerPass).to.equal(0);
        expect(didSomething).to.equal(4);
        done();
      }, 400);
    }, 999);
  });
});
