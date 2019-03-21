// const chai = require('chai');
// const chaiAsPromised = require('chai-as-promised');
// const AuthService = require('../../../../lib/services/authService/authService');
//
// const { expect } = chai;
//
// describe('services/node/peer', () => {
//   describe('#factory', () => {
//     it('should create AuthService instance without data', () => {
//       const res = new AuthService();
//       expect(res).to.be.instanceof(AuthService);
//     });
//     it('should AuthService has isValidTxId function', () => {
//       const authService = new AuthService();
//       const res = authService.isValidTxId;
//       expect(res).to.be.a('function');
//     });
//     it('should AuthService call disconnect function', () => {
//       const authService = new AuthService();
//       const res = authService.isValidTxId();
//       expect(res).to.be.equal(true); // hard coded value
//     });
//     it('should AuthService has getChallenge function', () => {
//       const authService = new AuthService();
//       const res = authService.getChallenge;
//       expect(res).to.be.a('function');
//     });
//     it('should not AuthService call getChallenge function without data', () => {
//       const authService = new AuthService();
//       const res = authService.getChallenge();
//       expect(res).to.be.a('string');
//     });
//     it('should AuthService call getChallenge function with data', () => {
//       const authService = new AuthService();
//       const res = authService.getChallenge('a');
//       expect(res).to.be.a('string');
//     });
//     it('should AuthService getChallenge return uniq data every time', () => {
//       const authService = new AuthService();
//       const res = authService.getChallenge();
//       const res2 = authService.getChallenge();
//       expect(res).to.not.equal(res2);
//     });
//     it('should be getUserObj function', () => {
//       const authService = new AuthService();
//       const res = authService.getUserObj;
//       expect(res).to.be.a('function');
//     });
//     it('should updateUsernameIndex return promise', () => {
//       const authService = new AuthService();
//       const res = authService.getUserObj();
//       expect(res).to.be.a('promise');
//     });
//     it('should AuthService has resolveChallenge function', () => {
//       const authService = new AuthService();
//       const res = authService.resolveChallenge;
//       expect(res).to.be.a('function');
//     });
//     it('should not AuthService call resolveChallenge function without data', () => {
//       // TODO: post a ticket
//       // TypeError: Cannot read property 'filter' of undefined
//       //     at getUserObj.then.then (../dapi/lib/services/authService/authService.js:50:37)
//       const authService = new AuthService();
//       const res = authService.resolveChallenge();
//       expect(res).to.be.a('promise');
//     });
//   });
// });
