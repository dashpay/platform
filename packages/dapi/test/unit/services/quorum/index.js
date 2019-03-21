// /* eslint-disable global-require */
// process.on('unhandledRejection', (up) => { throw up; });
// const chai = require('chai');
// const chaiAsPromised = require('chai-as-promised');
//
// chai.use(chaiAsPromised);
// const proxyquire = require('proxyquire');
// const ZmqClient = require('../../../../lib/api/dashcore/ZmqClient');
//
// const { expect } = chai;
//
// describe('services/quorum/index', () => {
//   const config = require('../../../../lib/config');
//   config.dashcore.rpc.port = 12345;
//   proxyquire('../../../../lib/api/dashcore/rpc', { '../../../config': config });
//   require('../../../../lib/api/dashcore/rpc');
//   const quorumService = require('../../../../lib/services/quorum/index');
//
//   describe('#factory', () => {
//     it('should quorum has start function', () => {
//       const res = quorumService.start;
//       expect(res).to.be.a('function');
//     });
//     it('should not quorum call start function without dashcoreZmqClient', () => {
//       expect(() => quorumService.start())
// .to.throw('Cannot read property \'topics\' of undefined');
//     });
//     it('should quorum call start function with dashcoreZmqClient', () => {
//       const zmqClient = new ZmqClient();
//       const res = quorumService.start(zmqClient);
//       expect(res).to.be.a('undefined');
//     });
//     it('should quorum has migrateClients function', () => {
//       const res = quorumService.migrateClients;
//       expect(res).to.be.a('function');
//     });
//     it('should quorum call migrateClients function', () => {
//       const res = quorumService.migrateClients();
//       expect(res).to.be.a('undefined');
//     });
//     it('should quorum has joinQuorum function', () => {
//       const res = quorumService.joinQuorum;
//       expect(res).to.be.a('function');
//     });
//     it('should quorum call joinQuorum function', () => {
//       const res = quorumService.joinQuorum();
//       expect(res).to.be.a('promise');
//     });
//     it('should quorum has getQuorumHash function', () => {
//       const res = quorumService.getQuorumHash;
//       expect(res).to.be.a('function');
//     });
//     it('should quorum call getQuorumHash function', () => {
//       const res = quorumService.getQuorumHash();
//       expect(res).to.be.a('promise');
//     });
//     it('should getQuorumHash rejected with invalid settings', async () => {
//       const res = quorumService.getQuorumHash();
//       await expect(res)
// .to.be.rejectedWith('Dash JSON-RPC: Request Error: connect ECONNREFUSED 127.0.0.1:12345');
//     });
//     it('should quorum has getQuorum function', () => {
//       const res = quorumService.getQuorum;
//       expect(res).to.be.a('function');
//     });
//     it('should quorum call getQuorum function', () => {
//       const res = quorumService.getQuorum();
//       expect(res).to.be.a('promise');
//     });
//     it('should getQuorum rejected with invalid settings', async () => {
//       const res = quorumService.getQuorum();
//       await expect(res)
// .to.be.rejectedWith('Dash JSON-RPC: Request Error: connect ECONNREFUSED 127.0.0.1:12345');
//     });
//     it('should quorum has isValidQuorum function', () => {
//       const res = quorumService.isValidQuorum;
//       expect(res).to.be.a('function');
//     });
//     it('should quorum call isValidQuorum function', () => {
//       const res = quorumService.isValidQuorum();
//       expect(res).to.be.a('promise');
//     });
//     it('should isValidQuorum rejected without data', async () => {
//       const res = quorumService.isValidQuorum();
//       await expect(res).to.be.rejectedWith('Cannot read property \'data\' of undefined');
//     });
//     it('should isValidQuorum rejected with invalid data', async () => {
//       const res = quorumService.isValidQuorum('data');
//       await expect(res).to.be.rejectedWith('Cannot read property \'txId\' of undefined');
//     });
//   });
// });
