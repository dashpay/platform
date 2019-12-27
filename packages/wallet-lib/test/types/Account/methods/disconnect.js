const { expect } = require('chai');
const connect = require('../../../../src/types/Account/methods/connect');
const disconnect = require('../../../../src/types/Account/methods/disconnect');
const DummyWorker = require('../../../fixtures/DummyWorker');

let transportConnected = false;
const emitted = [];

describe('Account - disconnect', () => {
  const self = {
    events: {
      emit: (eventName) => emitted.push(eventName),
    },
    transport: {
      isValid: true,
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
  self.plugins.workers.dummyWorker.events = self.events;
  connect.call(self);
  it('should disconnect to stream and worker', () => {
    expect(transportConnected).to.equal(true);
    disconnect.call(self);
    // console.log(self, transportConnected, emitted);
    expect(transportConnected).to.equal(false);
    expect(emitted).to.deep.equal([
      'WORKER/DUMMYWORKER/STARTED',
      'WORKER/DUMMYWORKER/STOPPED',
    ]);
  });
});
