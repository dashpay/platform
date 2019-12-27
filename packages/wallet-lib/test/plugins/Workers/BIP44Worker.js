// const { expect } = require('chai');
// const EventEmitter = require('events');
// const BIP44Worker = require('../../src/plugins/Workers/BIP44Worker');
//
//
// const opts = {
//   events: new EventEmitter(),
//   workerIntervalTime: 10000,
//   storage: {
//     store: {
//       wallets: {
//         abc: {
//           addresses: {
//             external: {},
//             internal: {},
//             misc: {},
//           },
//         },
//       },
//     },
//   },
//   walletId: 'abc',
// };
//
// opts.getAddress = () => false;
// opts.storage.getStore = () => opts.storage.store;
// let worker = null;
//
// describe('Plugins - BIP44Worker', function suite() {
//   this.timeout(60000);
//
//   it('should instantiate a new worker', () => {
//     worker = new BIP44Worker(opts);
//     expect(worker).to.not.equal(null);
//     expect(worker.workerRunning).to.equal(false);
//     expect(worker.workerPass).to.equal(0);
//     expect(worker.worker).to.equal(null);
//   });
//   it('should start the worker', (done) => {
//     worker.startWorker();
//     expect(worker.workerRunning).to.equal(false);
//     expect(worker.worker).to.not.equal(null);
//     expect(worker.workerPass).to.equal(0);
//
//     setTimeout(() => {
//       expect(worker.workerRunning).to.equal(false);
//       expect(worker.worker).to.not.equal(null);
//       expect(worker.workerPass).to.equal(1);
//       done();
//     }, 1000);
//   });
//   it('should execute afterwards (intervaL)', (done) => {
//     setTimeout(() => {
//       expect(worker.workerRunning).to.equal(false);
//       expect(worker.worker).to.not.equal(null);
//       expect(worker.workerPass).to.equal(2);
//       done();
//     }, worker.workerIntervalTime);
//   });
//   it('should restart when recall', () => {
//     worker.startWorker();
//     expect(worker.workerRunning).to.equal(false);
//     expect(worker.worker).to.not.equal(null);
//     expect(worker.workerPass).to.equal(0);
//   });
//   it('should allow to exec', () => {
//     expect(worker.workerRunning).to.equal(false);
//     expect(worker.worker).to.not.equal(null);
//     expect(worker.workerPass).to.equal(0);
//     worker.execWorker();
//     expect(worker.workerPass).to.equal(1);
//     worker.execWorker();
//     expect(worker.workerPass).to.equal(2);
//   });
//
//   it('should stop running after 42000', () => {
//     worker.workerPass = 42001;
//     expect(worker.execWorker()).to.equal(false);
//     worker.stopWorker();
//   });
// });
