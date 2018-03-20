// const should = require('should');

// describe('SPV - Init Chain', () => {
//   it('should initialise a chain with no data file', () => SDK.SPV.initChain(null, 'testnet')
//     .then((success) => {
//       success.should.be.true();
//     }));
//
//   it('chain should have correct genesis hash', () => {
//     SDK.SPV.getTipHash().should.equal('00000bafbc94add76cb75e2ec92894837288a481e5c005f6563d91623bf8bc2c');
//   });
//
//   it('should add 100 testnet blocks to the chain', () => SDK.Explorer.API.getBlockHeaders(1, 100)
//     .then((headers) => {
//       const height = SDK.SPV.addBlockHeaders(headers);
//       height.should.equal(101);
//     }));
//
//   it('should initialise a chain with existing data file of 100 blocks', () => {
//     // TODO
//   });
//
//   it('should verify coinbase tx of block 100 included by using merkle proofs', () => {
//     const txHash = '12b9fb0fb97105baf93ece60d28493a09f69bbc9834dd08ec8c5e1154198a41b';
//     return SDK.Explorer.API.getHashFromHeight(100)
//       .then(blockHash => SDK.SPV.validateTx(blockHash, txHash))
//       .then((isValid) => {
//         isValid.should.be.true();
//       });
//   });
// });
