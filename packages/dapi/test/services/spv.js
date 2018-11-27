// const chai = require('chai');
// const chaiAsPromised = require('chai-as-promised');
// const assert = require('assert');
// const { SpvService, getCorrectedHash,
// clearDisconnectedClientBloomFilters } = require('../../lib/services/spv');
//
// const { expect } = chai;
//
// describe('SPV', () => {
//   describe('#factory', () => {
//     it('should create SpvService instance without data', () => {
//       const res = new SpvService();
//       expect(res).to.be.instanceof(SpvService);
//     });
//     it('should SpvService has updateLastSeen function', () => {
//       const spvService = new SpvService();
//       const res = spvService.updateLastSeen;
//       expect(res).to.be.a('function');
//     });
//     it('should not SpvService call updateLastSeen function without filter', () => {
//       const spvService = new SpvService();
//       expect(() => spvService.updateLastSeen())
// .to.throw('Cannot set property \'lastSeen\' of undefined');
//     });
//     it('should SpvService has createNewClient function', () => {
//       const spvService = new SpvService();
//       const res = spvService.createNewClient;
//       expect(res).to.be.a('function');
//     });
//     it('should not SpvService call createNewClient function without filter', () => {
//       const spvService = new SpvService();
//       expect(() => spvService.createNewClient()).to.throw('Object argument required.');
//     });
//     it('should SpvService has initListeners function', () => {
//       const spvService = new SpvService();
//       const res = spvService.initListeners;
//       expect(res).to.be.a('function');
//     });
//     it('should not SpvService call initListeners function without filter', () => {
//       const spvService = new SpvService();
//       expect(() => spvService.initListeners()).to.throw('Object argument required.');
//     });
//     it('should SpvService has hasPeerInClients function', () => {
//       const spvService = new SpvService();
//       const res = spvService.hasPeerInClients;
//       expect(res).to.be.a('function');
//     });
//     it('should not SpvService call hasPeerInClients function without filter', () => {
//       const spvService = new SpvService();
//       expect(() => spvService.hasPeerInClients()).to.throw('Object argument required.');
//     });
//     it('should SpvService has getPeerFromClients function', () => {
//       const spvService = new SpvService();
//       const res = spvService.getPeerFromClients;
//       expect(res).to.be.a('function');
//     });
//     it('should not SpvService call getPeerFromClients function without filter', () => {
//       const spvService = new SpvService();
//       expect(() => spvService.getPeerFromClients()).to.throw('Object argument required.');
//     });
//     it('should SpvService has loadBloomFilter function', () => {
//       const spvService = new SpvService();
//       const res = spvService.loadBloomFilter;
//       expect(res).to.be.a('function');
//     });
//     it('should SpvService call loadBloomFilter function without filter', () => {
//       const spvService = new SpvService();
//       const res = spvService.loadBloomFilter('fakeFilter');
//       expect(res).to.be.a('promise');
//     });
//     it('should SpvService has clearBoomFilter function', () => {
//       const spvService = new SpvService();
//       const res = spvService.clearBoomFilter;
//       expect(res).to.be.a('function');
//     });
//     it('should not SpvService call clearBoomFilter function without filter', () => {
//       const spvService = new SpvService();
//       expect(() => spvService.clearBoomFilter()).to.throw('Object argument required.');
//     });
//     it('should SpvService has addToBloomFilter function', () => {
//       const spvService = new SpvService();
//       const res = spvService.addToBloomFilter;
//       expect(res).to.be.a('function');
//     });
//     it('should not SpvService call addToBloomFilter function without filter', () => {
//       const spvService = new SpvService();
//       expect(() => spvService.addToBloomFilter())
// .to.throw('Cannot set property \'lastSeen\' of undefined');
//     });
//     it('should SpvService has getData function', () => {
//       const spvService = new SpvService();
//       const res = spvService.getSpvData;
//       expect(res).to.be.a('function');
//     });
//     it('should not SpvService call getData function without filter', () => {
//       const spvService = new SpvService();
//       expect(() => spvService.getSpvData())
// .to.throw('Cannot set property \'lastSeen\' of undefined');
//     });
//     it('should SpvService has findDataForBlock function', () => {
//       const spvService = new SpvService();
//       const res = spvService.findDataForBlock;
//       expect(res).to.be.a('function');
//     });
//     it('should not SpvService call findDataForBlock function without filter', () => {
//       const spvService = new SpvService();
//       expect(() => spvService.findDataForBlock())
// .to.throw('Cannot set property \'lastSeen\' of undefined');
//     });
//   });
//   describe('getCorrectedHash', () => {
//     it('should return a corrected reversed hash object', () => {
//       const buffer = Buffer.from([
//         0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
//         1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
//       ]);
//       const expected = '0101010101010101010101010101010101010101010101010101010101010100';
//       const actual = getCorrectedHash(buffer);
//       assert.equal(actual, expected);
//     });
//   });
//   describe('clearDisconnectedClientBloomFilters', () => {
//     it('should return an empty array when the incoming clients array is empty', () => {
//       const expected = [];
//       const actual = clearDisconnectedClientBloomFilters({ clients: [] });
//       assert.deepEqual(actual, expected);
//     });
//     it('should return the list of clients remaining after removing those that have timed out',
// () => {
//       const client = lastSeen => ({
//         filter: 'filter',
//         peer: { messages: { FilterClear: () => {} }, sendMessage: () => { } },
//         lastSeen,
//       });
//       const currentTime = new Date(1529427556922);
//       const hasDisconnectedThresholdInMsec = 60000;
//       const clients = [
//         client(new Date(currentTime.getTime() - hasDisconnectedThresholdInMsec)),
//         client(currentTime),
//         client(new Date(currentTime - (hasDisconnectedThresholdInMsec + 1))),
//       ];
//       const expected = [client(currentTime)];
//       const actual = clearDisconnectedClientBloomFilters({
//         clients, currentTime, hasDisconnectedThresholdInMsec,
//       });
//       // TODO: How do you get assert to compare nested arrays correctly?
//       assert.deepEqual(actual[0].filter, expected[0].filter);
//     });
//   });
// });
