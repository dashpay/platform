const { expect } = require('chai');
const startWorker = require('./startWorker');

let testInterval = null;
const simulateChangeEvery = function (ms) {
  const self = this;
  testInterval = setInterval(() => {
    self.lastModified = Date.now();
  }, ms);
};
describe('Storage - startWorker', function suite() {
  this.timeout(60000);
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
    const self = {
      saveState: () => {
        saved += 1;
        self.lastSave = Date.now();
      },
      autosaveIntervalTime: 500,
      lastModified: Date.now(),
      lastSave: 0,
    };
    startWorker.call(self);
    simulateChangeEvery.call(self, 200);

    setTimeout(() => {
      clearInterval(self.interval);
      testInterval = clearInterval(testInterval);

      expect(saved < 11).to.be.equal(true);
      // First autosave + 9 induced changes
      // However it can be less as we do not hard force the place in the event loop (simple setInterval)
      res(expect(saved >= 8).to.be.equal(true));
    }, 5499);
  }));
});
