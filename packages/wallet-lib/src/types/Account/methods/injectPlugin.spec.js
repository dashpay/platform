const { expect } = require('chai');
const EventEmitter = require('events');
const FaultyWorker = require('../../../../fixtures/plugins/FaultyWorker');
const WorkingWorker = require('../../../../fixtures/plugins/WorkingWorker');
const injectPlugin = require('./injectPlugin');
const expectThrowsAsync = require('../../../utils/expectThrowsAsync');

describe('Account - injectPlugin', function suite() {
  this.timeout(12000);
  const parentEvents = new EventEmitter();
  const emitter = new EventEmitter();
  const mockedSelf = {
    plugins: {
      standard:{},
      workers: {},
      watchers:{}
    },
    storage:{},
    walletId: '123abc',
    parentEvents,
    on: emitter.on,
    emit: emitter.emit,
  }
  it('should prevent sensible access', async function () {
    const expectedException1 = 'Injection of plugin : storage Unallowed';
    await expectThrowsAsync(async () => await injectPlugin.call(mockedSelf, WorkingWorker), expectedException1);
  });
  it('should work', function (done) {
    // Time of exec is 10000 ms
    injectPlugin.call(mockedSelf, WorkingWorker, true).then(() => {
      expect(mockedSelf.plugins.workers['workingworker']).to.exist;
      expect(mockedSelf.storage.workingWorkerPass).to.equal(1);
    });

    setTimeout(() => {
      expect(mockedSelf.storage.workingWorkerPass).to.equal(2);
      mockedSelf.plugins.workers['workingworker'].stopWorker();

      done();
    }, 10000);
  });
  it('should handle faulty worker', async function () {
    const expectedException1 = 'Some reason.';
    await expectThrowsAsync(async () => await injectPlugin.call(mockedSelf, FaultyWorker, true), expectedException1);
      expect(mockedSelf.plugins.workers['faultyworker']).to.exist;
      expect(mockedSelf.plugins.workers['faultyworker'].worker).to.equal(null);
      expect(mockedSelf.plugins.workers['faultyworker'].isWorkerRunning).to.equal(false);
      expect(mockedSelf.plugins.workers['faultyworker'].state.started).to.equal(false);
  });
});
