const { expect } = require('chai');
const connect = require('./connect');
const disconnect = require('./disconnect');
const DummyWorker = require('../../../../fixtures/DummyWorker');

let transportConnected = false;
const emitted = [];

describe('Account - disconnect', function suite() {
  this.timeout(10000);
  const self = {
    emit: (eventName) => emitted.push(eventName),
    removeAllListeners: () => null,
    storage: {
      removeAllListeners: () => null,
      startWorker: () => null,
      saveState: () => null,
      stopWorker: () => null,
    },
    transport: {
      connect: () => { transportConnected = true; },
      disconnect: () => { transportConnected = false; },
    },
    plugins: {
      workers: {
        dummyWorker: new DummyWorker(),
      },
    },
  };
  // We simulate what injectPlugin does regarding events
  self.plugins.workers.dummyWorker.parentEvents = { on: self.on, emit: self.emit };
  connect.call(self);
  it('should disconnect to stream and worker', async () => {
    expect(transportConnected).to.equal(true);
    await disconnect.call(self);
    // console.log(self, transportConnected, emitted);
    expect(transportConnected).to.equal(false);
    expect(emitted).to.deep.equal([
      'WORKER/DUMMYWORKER/STARTED',
      'WORKER/DUMMYWORKER/EXECUTED',
      'WORKER/DUMMYWORKER/STOPPED',
    ]);
  });
});
