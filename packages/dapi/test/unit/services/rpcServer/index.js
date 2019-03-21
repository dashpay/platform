// const chai = require('chai');
// const chaiAsPromised = require('chai-as-promised');
//
// chai.use(chaiAsPromised);
// const { expect } = chai;
// const {
// createCommands, start, createRegtestCommands
// } = require('../../../../lib/rpcServer/index');
//
// describe('lib/rpcServer/index', () => {
//   describe('#factory', () => {
//     it('should createCommands return function', () => {
//       const res = createCommands;
//       expect(res).to.be.a('function');
//     });
//     it('should call createCommands be instance of Object', () => {
//       const res = createCommands();
//       expect(res).to.be.instanceof(Object);
//     });
//     it('should createCommands.estimateFee be a promise', () => {
//       const res = createCommands();
//       expect(res.estimateFee()).to.be.a('promise');
//     });
//     it('should createCommands generate a list of function', async () => {
//       const res = createCommands();
//       await expect(res.estimateFee()).to.be.rejectedWith('params should be object');
//     });
//     ['estimateFee', 'getAddressSummary', 'getAddressTotalReceived', 'getAddressTotalSent',
//       'getAddressUnconfirmedBalance',
// 'getBalance', 'getBestBlockHeight', 'getBlockHash', 'getBlocks',
//       'getHistoricBlockchainDataSyncStatus',
// 'getMNList', 'getPeerDataSyncStatus', 'getRawBlock', 'getStatus',
//       'getTransactionById', 'getTransactionsByAddress', 'getUser', 'getUTXO', 'getBlockHeaders',
//       'sendRawTransaction', 'sendRawTransition',
//       'fetchDapContract', 'searchUsers', 'sendRawIxTransaction'].forEach((f) => {
//       it('should createCommands.getAddressSummary be a promise', () => {
//         const res = createCommands();
//         expect(res[f]).to.be.a('function');
//       });
//     });
//     ['estimateFee', 'getAddressSummary', 'getAddressTotalReceived', 'getAddressTotalSent',
//       'getAddressUnconfirmedBalance',
// 'getBalance', 'getBestBlockHeight', 'getBlockHash', 'getBlocks',
//       'getMNList', 'getRawBlock',
// 'getStatus', 'getTransactionById', 'getTransactionsByAddress', 'getUser',
//       'getUTXO', 'getBlockHeaders', 'sendRawTransaction', 'sendRawTransition',
//       'fetchDapContract', 'searchUsers'].forEach((f) => {
//       it('should call items from createCommands list', () => {
//         const res = createCommands();
//         expect(res[f]()).to.be.a('promise');
//       });
//     });
//     it('should not call createCommands.getAddressSummary without params', async () => {
//       const res = createCommands();
//       await expect(res.getAddressSummary()).to.be.rejectedWith('params should be object');
//     });
//     it('should start return function', () => {
//       const res = start;
//       expect(res).to.be.a('function');
//     });
//     it('should call start return undefined', () => {
//       const res = start();
//       expect(res).to.be.a('undefined');
//     });
//   });
// });
