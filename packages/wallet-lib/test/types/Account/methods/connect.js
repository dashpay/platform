const { expect } = require('chai');
const connect = require('../../../../src/types/Account/methods/connect');
const DummyWorker = require('../../../fixtures/DummyWorker');

let transportConnected = false;
const emitted = [];

describe('Account - connect', () => {
  it('should connect to transport and worker', () => {
    const self = {
      events: {
        emit: (eventName) => emitted.push(eventName),
      },
      transport: {
        isValid: true,
        connect: () => { transportConnected = true; },
      },
      plugins: {
        workers: {
          dummyWorker: new DummyWorker(),
        },
      },
    };

    // We simulate what injectPlugin does regarding events
    self.plugins.workers.dummyWorker.events = self.events;

    expect(connect.call(self)).to.equal(true);
    expect(emitted).to.deep.equal(['WORKER/DUMMYWORKER/STARTED']);
    expect(transportConnected).to.deep.equal(true);

    // We need to stop the worker
    self.plugins.workers.dummyWorker.stopWorker();
  });
});
