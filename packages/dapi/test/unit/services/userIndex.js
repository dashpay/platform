// const chai = require('chai');
// const userIndex = require('../../../lib/services/userIndex');
// const ZmqClient = require('../../../lib/api/dashcore/ZmqClient');
//
// const { expect } = chai;
//
// describe('userIndex', () => {
//   describe('#factory', () => {
//     it('should processBlock be hidden', () => {
//       const res = userIndex.processBlock;
//       expect(res).to.be.a('undefined');
//     });
//     it('should be updateUsernameIndex function', () => {
//       const res = userIndex.updateUsernameIndex;
//       expect(res).to.be.a('function');
//     });
//
//     it('should be searchUsernames function', () => {
//       const res = userIndex.searchUsernames;
//       expect(res).to.be.a('function');
//     });
//
//     it('should be getUserById function', () => {
//       const res = userIndex.getUserById;
//       expect(res).to.be.a('function');
//     });
//
//     it('should be subscribeToZmq function', () => {
//       const res = userIndex.subscribeToZmq;
//       expect(res).to.be.a('function');
//     });
//     // it('should updateUsernameIndex return promise', () => {
//     //   const res = userIndex.updateUsernameIndex();
//     //   expect(res).to.be.a('promise');
//     // });
//     it('should searchUsernames return promise', () => {
//       const res = userIndex.searchUsernames('fake');
//       expect(res).to.be.a('promise');
//     });
//     it('should subscribeToZmq return error when zmqClient invalid', async () => {
//       expect(() => userIndex.subscribeToZmq('fake'))
// .to.throw('Cannot read property \'hashblock\' of undefined');
//     });
//     it('should subscribeToZmq be called with valid zmqClient', async () => {
//       const zmqClient = new ZmqClient();
//       const res = userIndex.subscribeToZmq(zmqClient);
//       expect(res).to.be.a('undefined');
//     });
//     it('should getUserById return undefined with non-existing user', async () => {
//       const res = userIndex.getUserById('fake');
//       expect(res).to.be.a('undefined');
//     });
//     // it('should updateUsernameIndex return promise when isUpdating = true', () => {
//     //   userIndex.isUpdating = true;
//     //   const res = userIndex.updateUsernameIndex();
//     //   expect(res).to.be.a('promise');
//     // });
//   });
// });
