// const { expect } = require('chai');
// const EventEmitter = require('events');
// const SyncWorkerSyncWorker = require('../../src/plugins/Workers/SyncWorker');
// const Transporter = require('../../src/transports/Transporter');
//
// const opts = {
//   workerIntervalTime: 1000,
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
//   transporter: new Transporter(),
//   walletId: 'abc',
// };
//
// opts.fetchStatus = () => false;
// opts.fetchAddressInfo = () => false;
// opts.fetchTransactionInfo = () => false;
// opts.storage.getStore = () => opts.storage.store;
// let worker = null;
//
// describe('Plugins - SyncWorker', function suite() {
//   this.timeout(60000);
//
//   it('should instantiate a new worker', () => {
//     worker = new SyncWorker(opts);
//     const now = Date.now();
//     const expectedFetchthreeshold = now - (10 * 60 * 1000);
//     expect(worker.fetchThreeshold).to.below(expectedFetchthreeshold + 100);
//     expect(worker.fetchThreeshold).to.above(expectedFetchthreeshold - 100);
//     expect(worker).to.not.equal(null);
//     expect(worker.workerRunning).to.equal(false);
//     expect(worker.workerPass).to.equal(0);
//     expect(worker.worker).to.equal(null);
//   });
//   it('should be able to have specific threeholdMs opt', () => {
//     const worker2 = new SyncWorker({ threesholdMs: 60 * 1000 });
//     const now = Date.now();
//     const expectedFetchthreeshold = now - (1 * 60 * 1000);
//     expect(worker2.fetchThreeshold).to.below(expectedFetchthreeshold + 100);
//     expect(worker2.fetchThreeshold).to.above(expectedFetchthreeshold - 100);
//     expect(worker2).to.not.equal(null);
//     expect(worker2.workerRunning).to.equal(false);
//     expect(worker2.workerPass).to.equal(0);
//     expect(worker2.worker).to.equal(null);
//     worker2.stopWorker();
//   });
//   it('should start the worker', (done) => {
//     worker.startWorker();
//     expect(worker.workerRunning).to.equal(false);
//     expect(worker.worker).to.not.equal(null);
//     expect(worker.workerPass).to.equal(0);
//
//     setTimeout(() => {
//       expect(worker.workerRunning).to.equal(true);
//       expect(worker.worker).to.not.equal(null);
//       expect(worker.workerPass).to.equal(0);
//       setTimeout(() => {
//         expect(worker.workerRunning).to.equal(false);
//         expect(worker.workerPass).to.equal(1);
//         done();
//       }, 100);
//     }, opts.workerIntervalTime);
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
//   it('should allow to exec', (done) => {
//     expect(worker.workerRunning).to.equal(false);
//     expect(worker.worker).to.not.equal(null);
//     expect(worker.workerPass).to.equal(0);
//     const exec = worker.execWorker();
//     expect(worker.workerRunning).to.equal(true);
//     expect(worker.workerPass).to.equal(0);
//     worker
//       .execWorker()
//       .then((execAgain) => {
//         expect(worker.workerRunning).to.equal(true);
//         expect(execAgain).to.equal(false);
//         exec
//           .then(() => {
//             expect(worker.workerPass).to.equal(1);
//             setTimeout(() => {
//               expect(worker.workerRunning).to.equal(false);
//               expect(worker.worker).to.not.equal(null);
//               expect(worker.workerPass).to.equal(1);
//               done();
//             }, 100);
//           });
//       });
//   });
//
//   it('should stop running after 42000', async () => {
//     worker.workerPass = 42001;
//     const exec = await worker.execWorker();
//     expect(exec).to.equal(false);
//     worker.stopWorker();
//   });
// });
