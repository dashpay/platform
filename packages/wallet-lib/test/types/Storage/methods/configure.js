const { expect } = require('chai');
const configure = require('../../../../src/types/Storage/methods/configure');

const noop = () => {};

describe('Storage - configure', async () => {
  it('should set save/rehydrate settings', () => {
    let rehydrated = 0;

    const self = {
      events: {
        emit: noop,
      },
      autosaveIntervalTime: 1000,
      startWorker: noop,
      rehydrateState: () => (rehydrated += 1),
      rehydrate: true,
      autosave: true,
    };
    expect(rehydrated).to.equal(0);
    return configure
      .call(self)
      .then(() => expect(self.autosave).to.equal(true))
      .then(() => expect(self.rehydrate).to.equal(true))
      .then(() => expect(rehydrated).to.equal(1))
      .then(() => configure
        .call(self, { rehydrate: false, autosave: false })
        .then(() => expect(self.autosave).to.equal(false))
        .then(() => expect(self.rehydrate).to.equal(false))
        .then(() => expect(rehydrated).to.equal(1)));
  });
  it('should successfully emit', () => {
    const emitted = [];
    const self = {
      events: {
        emit: emitType => (emitted.push(emitType)),
      },
      autosaveIntervalTime: 1000,
      startWorker: noop,
    };
    expect(emitted.length).to.equal(0);

    return configure
      .call(self)
      .then(() => expect(emitted).to.deep.equal(['CONFIGURED']));
  });
  it('should start the autosave worker if autosave is true', () => {
    let workerStarted = false;
    const self = {
      rehydrate: false,
      autosave: false,
      autosaveIntervalTime: 1000,
      startWorker: () => { workerStarted = true; },
      events: {
        emit: noop,
      },
    };

    return configure
      .call(self)
      .then(() => expect(workerStarted).to.equal(false))
      .then(() => configure
        .call(self, { autosave: true })
        .then(() => expect(workerStarted).to.equal(true)));
  });
});
