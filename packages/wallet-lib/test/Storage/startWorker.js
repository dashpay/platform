const { expect } = require('chai');
const startWorker = require('../../src/Storage/startWorker');

const noop = () => {};

describe('Storage - startWorker', function suite() {
  it('should set an interval', () => {
    const defaultIntervalValue = 10000;
    const self = {
      autosaveIntervalTime: defaultIntervalValue,
    };
    startWorker.call(self);
    expect(self.interval.constructor.name).to.be.equal('Timeout');
    expect(self.interval._repeat).to.be.equal(defaultIntervalValue); // Timeout are null btw
    clearInterval(self.interval);
  });
  it('should works', async () => new Promise((res) => {
    let saved = 0;
    let testInterval = null;
    const self = {
      saveState: () => { saved += 1; self.lastSave = Date.now(); },
      autosaveIntervalTime: 10,
      lastModified: Date.now(),
      lastSave: 0,
    };
    startWorker.call(self);

    const simulateChangeEvery = (ms) => {
      testInterval = setInterval(() => {
        self.lastModified = Date.now();
      }, ms);
    };

    simulateChangeEvery(20);

    setTimeout(() => {
      clearInterval(self.interval);
      testInterval = clearInterval(testInterval);

      // First autosave + 4 induced changes
      res(expect(saved).to.be.equal(5));
    }, 100);
  }));
});
