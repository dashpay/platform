// const { expect } = require('chai');
// const {Mnemonic} = require('@dashevo/dashcore-lib');
// const { HDPrivateKey } = require('@dashevo/dashcore-lib');

// const passphase = require('./fixtures').mnemonicString1;
// const mnemonic = new Mnemonic(passphase).toSeed();
// const privateHDKey = new HDPrivateKey.fromSeed(mnemonic);
// const { getBloomFilter } = require('../src/index');

// privateHDKey.derive('m/1');

// describe('Bloom Fiters', () => {
//   it('should...', (done) => {
// const noElements = 10;
// const privKeySeeds = [];
//
// for (let i = 0; i < noElements; i += 1) {
//   privKeySeeds.push(privateHDKey.derive(`m/1/${i}`).privateKey.toWIF());
// }
//
// const fpRate = 0.1;
// const filter = getBloomFilter(privKeySeeds, fpRate);
//
// expect(filter.nFlags === 1);
// expect(filter.nHashFuncs === 2);
// expect(filter.nTweak === 0);
// expect(filter.vData.length === 5);
// expect(filter.vData.toString() === '1,0,0,0,1');
// done();
// });
// });
