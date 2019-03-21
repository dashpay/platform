// const chai = require('chai');
// const chaiAsPromised = require('chai-as-promised');
// const sinon = require('sinon');
// const request = require('request-promise-native');
//
// chai.use(chaiAsPromised);
// const { expect } = chai;
//
// const insight = require('../../../lib/api/insight/index');
// const config = require('../../../lib/config/index');
//
// const URI = config.insightUri;
//
// let requestStub;
// const testBlockHeight = 123;
// const testAddress = {
//   addrStr: 'ygPcCwVy7Fxg7ruxZzqVYdPLtvw7auHAFh',
//   balance: 999.81550001,
//   balanceSat: 99981550001,
//   totalReceived: 9998.25310005,
//   totalReceivedSat: 999825310005,
//   totalSent: 8998.43760004,
//   totalSentSat: 899843760004,
//   unconfirmedBalance: 0,
//   unconfirmedBalanceSat: 0,
//   unconfirmedTxApperances: 0,
//   txApperances: 20,
//   transactions: [
//     '1396bc62950f8b3a3811f1134b64816273c72d19d9641d747cfd55bea9e120bd',
//     '21b2e4fbab4faa2bd002510d048673096fdd0798f51cb03182e60af99148cc8c',
//     'c7650e7efa0586de0816b9d59ef57079cdc4718242a4f1564d885c514a12e593',
//     'b33987dfe16b49466cecc855367d5fed771c5e66c81a0ef803cd151cb71ec6a3',
//     'cdab098edcfaab324ec28c817e372bcc2d6de8f4b05eb8b4e1d076062081cc95',
//     'f0b8eed510a398100b109b641c4d92b5500b1ed43a6b6838b7d3f40166fdfa85',
//     '6304d380f6fca41db11e34da36ff412fcb1a63a054a836cac05611dc84787fd6',
//     '0099c9ef8d652aed28e39af13780f3ad237bde0b5015852ff3f77f24c945d5fb',
//     '543c16fad42c25fd9e4f69053ae89f9da8580dba34d4d02df556c913af05a2f9',
//     '3db29d651dbb2d6ff32d4cb491dfbd5024a570ddc41a84e14c7e480e75a8fc22',
//     '84aad8e3ede9374049fe4e209b3228ca3e37b0b4c533185f3bb615394cb1b66c',
//     '830ac550a52a82b67d0d932dd17c0562735b606bbb63169770f86f62ade668be',
//     'b74cb1febdc8e3e29283aa954e631b5e07729c7cb552c379745dfb6a89bfe2b9',
//     '1c8c8179d4e7cba136a5c136e80b597f938ded53dd2ffea39e6aa82ac3edd8d1',
//     'acab2ede2411fc16467c6bfb52268e5330dfe9c3e0694a8d2aaccb007d3b2d51',
//     'df1a4f25f43df9262eafd32d7c856ee586cbc3450c76791627d650dfe665e6fb',
//     'c98efa2b6dc25ac1ee652456fd9d2aa1123b040c1d708cc45864b6a51c8ba36a',
//     '68bf284865c2e351cd9b385178d61e3c3dd114a8a48b2bce4b6506d553ca398b',
//     'f6c7b417d65525a003331ad5da8a2634149773720abfb91ff3e3fe7946c2d74f',
//     '82b1e2fe3e751583a6e6ba17fb5aa9f11df6a42f0a61a5c70717dd574c9df085',
//   ],
// };
//
// const textUtxo = [
//   {
//     address: 'ygPcCwVy7Fxg7ruxZzqVYdPLtvw7auHAFh',
//     txid: '42c56c8ec8e2cdf97a56bf6290c43811ff59181a184b70ebbe3cb66b2970ced2',
//     vout: 1,
//     scriptPubKey: '76a914dc2bfda564dc6217c55c842d65cc0242e095d2d788ac',
//     amount: 999.81530001,
//     satoshis: 99981530001,
//     confirmations: 0,
//     ts: 1529847344,
//   },
// ];
//
// const testStatus = {
//   info: {
//     blocks: {
//       info: {
//         version: 120300,
//         insightversion: '0.6.1',
//         protocolversion: 70209,
//         blocks: 278,
//         timeoffset: 0,
//         connections: 0,
//         proxy: '',
//         difficulty: 4.656542373906925e-10,
//         testnet: false,
//         relayfee: 0.00001,
//         errors:
// "Warning: Unknown block versions being mined! It's possible unknown rules are in effect",
//         network: 'testnet',
//       },
//     },
//   },
// };
//
// const testSync = {
//   status: 'finished',
//   blockChainHeight: 278,
//   syncPercentage: 100,
//   height: 278,
//   error: null,
//   type: 'bitcore node',
// };
//
// const testHashFromHeight = {
//   blockHash: {
//     blockHash: '345e794ba8ede65e974f09c0cbf211b1344c2e02172fde7087a2f251c8e39b17',
//   },
// };
//
// const testPeer = {
//   connected: true,
//   host: '127.0.0.1',
//   port: null,
// };
//
// const testBlockHeaders = {
//   headers: [
//     {
//       hash: '29490a67dd816306090b69bd2a6d2ca12522010e50438357c3fb06f20bf0e2df',
//       version: 536875008,
//       confirmations: 278,
//       height: 1,
//       chainWork: '0000000000000000000000000000000000000000000000000000000000000004',
//       prevHash: '000008ca1832a4baf228eb1553c03d3a2c8e02399550dd6ea8d65cec3ef23d2e',
//       nextHash: '10cfb7c41df6dadc00baf9f43fd3e202968289521d49f014aaeec3d0c74dd402',
//       merkleRoot: 'c2b409ae7eb315acccb6d85520d971324fd94de8ce5d77ef5727f18a4833d9f1',
//       time: 1529847184,
//       medianTime: 1529847184,
//       nonce: 0,
//       bits: '207fffff',
//       difficulty: 4.656542373906925e-10,
//     },
//     {
//       hash: '10cfb7c41df6dadc00baf9f43fd3e202968289521d49f014aaeec3d0c74dd402',
//       version: 536875008,
//       confirmations: 277,
//       height: 2,
//       chainWork: '0000000000000000000000000000000000000000000000000000000000000006',
//       prevHash: '29490a67dd816306090b69bd2a6d2ca12522010e50438357c3fb06f20bf0e2df',
//       nextHash: '59d9965df877570333ac149ef9b46d5d0892d4d7850edefb7f404762430477b1',
//       merkleRoot: '3cd699c1d744a033826922d8ffa67407ed01b01a92a9147f0317ca97d82bc4f7',
//       time: 1529847185,
//       medianTime: 1529847184,
//       nonce: 0,
//       bits: '207fffff',
//       difficulty: 4.656542373906925e-10,
//     },
//   ],
// };
//
// // TODO add tests for: https://dashpay.atlassian.net/browse/EV-939, https://dashpay.atlassian.net/browse/EV-940
// describe('Insight', () => {
//   afterEach(() => {
//     requestStub.restore();
//   });
//
//   describe('#getUser', () => {
//     const validUserData = {
//       result: {
//         uname: 'Alice',
//         regtxid: 'b65115c453394fd309582ddae07a53453f1481fdb1b637d20cec1f0baac1f6c3',
//         pubkey: '02cc389b4dbbe122e3842b4f6c07791801eb4c4d56cff48f6851cd873559eed8b3',
//         credits: 1000000,
//         subtx: [
//           'b65115c453394fd309582ddae07a53453f1481fdb1b637d20cec1f0baac1f6c3',
//         ],
//         state: 'open',
//       },
//     };
//     beforeEach(() => {
//       requestStub = sinon.stub(request, 'get');
//       requestStub.rejects(new Error('user Alice not found. Code:-1'));
//       requestStub
//         .withArgs(`${URI}/getuser/Alice`)
//         .returns(new Promise(resolve => resolve(validUserData)));
//     });
//
//     it('Should return user if such user exists on blockchain', async () => {
//       const user = await insight.getUser('Alice');
//       expect(user.uname).to.be.equal('Alice');
//       expect(user.regtxid).to.be.equal(validUserData.result.regtxid);
//       expect(user.pubkey).to.be.equal(validUserData.result.pubkey);
//       expect(user.credits).to.be.equal(validUserData.result.credits);
//       expect(user.subtx).to.be.equal(validUserData.result.subtx);
//       expect(user.state).to.be.equal(validUserData.result.state);
//     });
//
//     it('Should return error if user not found', () => expect(insight.getUser('Bob'))
// .to.be.rejectedWith('user Alice not found. Code:-1'));
//   });
//
//   describe('#getBestBlockHeight', () => {
//     const resp = {
//       height: testBlockHeight,
//     };
//     beforeEach(() => {
//       requestStub = sinon.stub(request, 'get');
//       requestStub.rejects(new Error('not defined'));
//       requestStub
//         .withArgs(`${URI}/bestBlockHeight`)
//         .returns(new Promise(resolve => resolve(resp)));
//     });
//
//
//     it('Should return bestBlockHeight', async () => {
//       const bestBlockHeight = await insight.getBestBlockHeight();
//       expect(bestBlockHeight).to.be.equal(resp.height);
//     });
//   });
//
//   describe('#getBlockHash', () => {
//     const resp = {
//       blockHash: '3e04e2ecaeff74fee1edc4f8b4c8415ade5fe29ef96d0c754b8b7ff461d9c6da',
//     };
//     beforeEach(() => {
//       requestStub = sinon.stub(request, 'get');
//       requestStub.rejects(new Error('Block height out of range. Code:-8'));
//       requestStub
//         .withArgs(`${URI}/block-index/${testBlockHeight}`)
//         .returns(new Promise(resolve => resolve(resp)));
//     });
//
//
//     it('Should return getBlockHash', async () => {
//       const blockHeight = await insight.getBlockHash(testBlockHeight);
//       expect(blockHeight).to.be.equal(resp.blockHash);
//     });
//
//     [-1, 123456789].forEach((bh) => {
//       it('Should return error if blockHeight  out of range',
// () => expect(insight.getBlockHash(bh)).to.be.rejectedWith('Block height out of range. Code:-8'));
//     });
//
//
//     ['str', false].forEach((bh) => {
//       it(
//         'Should return error if blockHeight  out of range',
//         () => {
//           requestStub.rejects(new Error('JSON value is not an integer as expected. Code:-1'));
//           expect(insight.getBlockHash(bh))
// .to.be.rejectedWith('JSON value is not an integer as expected. Code:-1');
//         },
//       );
//     });
//   });
//
//   describe('#getAddressTotalReceived', () => {
//     beforeEach(() => {
//       requestStub = sinon.stub(request, 'get');
//       requestStub.rejects(new Error('Invalid address: Checksum mismatch. Code:1'));
//       requestStub
//         .withArgs(`${URI}/addr/${testAddress.addrStr}/totalReceived`)
//         .returns(new Promise(resolve => resolve(testAddress)));
//     });
//
//
//     it('Should return getAddressTotalReceived', async () => {
//       const totalReceived = await insight.getAddressTotalReceived(testAddress.addrStr);
//       expect(totalReceived).to.be.equal(testAddress.totalReceived);
//     });
//
//     [testAddress.addrStr.substring(0, testAddress.addrStr.length - 1),
//       testAddress.addrStr.substring(1, testAddress.addrStr.length), 123456789].forEach((ad) => {
//       it('Should return error if Invalid address: Checksum mismatch',
// () => expect(insight.getAddressTotalReceived(ad))
// .to.be.rejectedWith('Invalid address: Checksum mismatch. Code:1'));
//     });
//
//
//     ['str', false].forEach((ad) => {
//       it(
//         'Should return error if Invalid address: Input string too short',
//         () => {
//           requestStub.rejects(new Error('Invalid address: Input string too short. Code:1'));
//           expect(insight.getAddressTotalReceived(ad))
// .to.be.rejectedWith('Invalid address: Input string too short. Code:1');
//         },
//       );
//     });
//
//     ['0', '0x'].forEach((ad) => {
//       it(
//         'Should return error if Invalid address: Non-base58 character',
//         () => {
//           requestStub.rejects(new Error('Invalid address: Non-base58 character. Code:1'));
//           expect(insight.getAddressTotalReceived(ad))
// .to.be.rejectedWith('Invalid address: Non-base58 character. Code:1');
//         },
//       );
//     });
//   });
//
//   describe('#getAddressTotalSent', () => {
//     beforeEach(() => {
//       requestStub = sinon.stub(request, 'get');
//       requestStub.rejects(new Error('Invalid address: Checksum mismatch. Code:1'));
//       requestStub
//         .withArgs(`${URI}/addr/${testAddress.addrStr}/totalSent`)
//         .returns(new Promise(resolve => resolve(testAddress.totalSent)));
//     });
//
//
//     it('Should return getAddressTotalSent', async () => {
//       const totalSent = await insight.getAddressTotalSent(testAddress.addrStr);
//       expect(totalSent).to.be.equal(testAddress.totalSent);
//     });
//
//     [testAddress.addrStr.substring(0, testAddress.addrStr.length - 1),
//       testAddress.addrStr.substring(1, testAddress.addrStr.length), 123456789].forEach((ad) => {
//       it('Should return error if Invalid address: Checksum mismatch',
// () => expect(insight.getAddressTotalSent(ad))
//         .to.be.rejectedWith('Invalid address: Checksum mismatch. Code:1'));
//     });
//
//
//     ['str', false].forEach((ad) => {
//       it(
//         'Should return error if Invalid address: Input string too short',
//         () => {
//           requestStub.rejects(new Error('Invalid address: Input string too short. Code:1'));
//           expect(insight.getAddressTotalReceived(ad))
// .to.be.rejectedWith('Invalid address: Input string too short. Code:1');
//         },
//       );
//     });
//
//     ['0', '0x'].forEach((ad) => {
//       it(
//         'Should return error if Invalid address: Non-base58 character',
//         () => {
//           requestStub.rejects(new Error('Invalid address: Non-base58 character. Code:1'));
//           expect(insight.getAddressTotalSent(ad))
// .to.be.rejectedWith('Invalid address: Non-base58 character. Code:1');
//         },
//       );
//     });
//   });
//
//   describe('#getAddressUnconfirmedBalance', () => {
//     beforeEach(() => {
//       requestStub = sinon.stub(request, 'get');
//       requestStub.rejects(new Error('Invalid address: Checksum mismatch. Code:1'));
//       requestStub
//         .withArgs(`${URI}/addr/${testAddress.addrStr}/unconfirmedBalance`)
//         .returns(new Promise(resolve => resolve(testAddress.unconfirmedBalance)));
//     });
//
//
//     it('Should return getAddressUnconfirmedBalance', async () => {
//       const unconfirmedBalance = await insight.getAddressUnconfirmedBalance(testAddress.addrStr);
//       expect(unconfirmedBalance).to.be.equal(testAddress.unconfirmedBalance);
//     });
//
//     [testAddress.addrStr.substring(0, testAddress.addrStr.length - 1),
//       testAddress.addrStr.substring(1, testAddress.addrStr.length), 123456789].forEach((ad) => {
//       it('Should return error if Invalid address: Checksum mismatch',
// () => expect(insight.getAddressUnconfirmedBalance(ad))
//         .to.be.rejectedWith('Invalid address: Checksum mismatch. Code:1'));
//     });
//
//
//     ['str', false].forEach((ad) => {
//       it(
//         'Should return error if Invalid address: Input string too short',
//         () => {
//           requestStub.rejects(new Error('Invalid address: Input string too short. Code:1'));
//           expect(insight.getAddressUnconfirmedBalance(ad))
// .to.be.rejectedWith('Invalid address: Input string too short. Code:1');
//         },
//       );
//     });
//
//     ['0', '0x'].forEach((ad) => {
//       it(
//         'Should return error if blockHeight Invalid address: Non-base58 characte',
//         () => {
//           requestStub.rejects(new Error('Invalid address: Non-base58 character. Code:1'));
//           expect(insight.getAddressUnconfirmedBalance(ad))
// .to.be.rejectedWith('Invalid address: Non-base58 character. Code:1');
//         },
//       );
//     });
//   });
//
//   describe('#getAddressSummary', () => {
//     beforeEach(() => {
//       requestStub = sinon.stub(request, 'get');
//       requestStub.rejects(new Error('Invalid address: Checksum mismatch. Code:1'));
//       requestStub
//         .withArgs(`${URI}/addr/${testAddress.addrStr}`)
//         .returns(new Promise(resolve => resolve(testAddress)));
//     });
//
//
//     it('Should return getAddressSummary', async () => {
//       const unconfirmedBalance = await insight.getAddressSummary(testAddress.addrStr);
//       expect(unconfirmedBalance).to.be.equal(testAddress);
//     });
//
//     [testAddress.addrStr.substring(0, testAddress.addrStr.length - 1),
//       testAddress.addrStr.substring(1, testAddress.addrStr.length), 123456789].forEach((ad) => {
//       it('Should return error if Invalid address: Checksum mismatch',
// () => expect(insight.getAddressSummary(ad))
//         .to.be.rejectedWith('Invalid address: Checksum mismatch. Code:1'));
//     });
//
//
//     ['str', false].forEach((ad) => {
//       it(
//         'Should return error if Invalid address: Input string too short',
//         () => {
//           requestStub.rejects(new Error('Invalid address: Input string too short. Code:1'));
//           expect(insight.getAddressSummary(ad))
// .to.be.rejectedWith('Invalid address: Input string too short. Code:1');
//         },
//       );
//     });
//
//     ['0', '0x'].forEach((ad) => {
//       it(
//         'Should return error if blockHeight Invalid address: Non-base58 character',
//         () => {
//           requestStub.rejects(new Error('Invalid address: Non-base58 character. Code:1'));
//           expect(insight.getAddressSummary(ad))
// .to.be.rejectedWith('Invalid address: Non-base58 character. Code:1');
//         },
//       );
//     });
//   });
//
//   describe('#getTransactionsByAddress', () => {
//     beforeEach(() => {
//       requestStub = sinon.stub(request, 'get');
//       requestStub.rejects(new Error('Invalid address: Checksum mismatch. Code:1'));
//       requestStub
//         .withArgs(`${URI}/addrs/${testAddress.addrStr}/txs`)
//         .returns(new Promise(resolve => resolve(testAddress.transactions)));
//     });
//
//     it('Should return getTransactionsByAddress', async () => {
//       const transactionsByAddress = await insight.getTransactionsByAddress(testAddress.addrStr);
//       expect(transactionsByAddress).to.be.equal(testAddress.transactions);
//     });
//
//     [testAddress.addrStr.substring(0, testAddress.addrStr.length - 1),
//       testAddress.addrStr.substring(1, testAddress.addrStr.length), 123456789].forEach((ad) => {
//       it('Should return error if Invalid address: Checksum mismatch',
// () => expect(insight.getTransactionsByAddress(ad))
//         .to.be.rejectedWith('Invalid address: Checksum mismatch. Code:1'));
//     });
//
//
//     ['str', false].forEach((ad) => {
//       it(
//         'Should return error if Invalid address: Input string too short',
//         () => {
//           requestStub.rejects(new Error('Invalid address: Input string too short. Code:1'));
//           expect(insight.getTransactionsByAddress(ad))
// .to.be.rejectedWith('Invalid address: Input string too short. Code:1');
//         },
//       );
//     });
//
//     ['0', '0x'].forEach((ad) => {
//       it(
//         'Should return error if blockHeight Invalid address: Non-base58 character',
//         () => {
//           requestStub.rejects(new Error('Invalid address: Non-base58 character. Code:1'));
//           expect(insight.getTransactionsByAddress(ad))
// .to.be.rejectedWith('Invalid address: Non-base58 character. Code:1');
//         },
//       );
//     });
//   });
//
//   describe('#getBalance', () => {
//     beforeEach(() => {
//       requestStub = sinon.stub(request, 'get');
//       requestStub.rejects(new Error('Invalid address: Checksum mismatch. Code:1'));
//       requestStub
//         .withArgs(`${URI}/addr/${testAddress.addrStr}/balance`)
//         .returns(new Promise(resolve => resolve(testAddress.balance)));
//     });
//
//     it('Should return getBalance', async () => {
//       const balance = await insight.getBalance(testAddress.addrStr);
//       expect(balance).to.be.equal(testAddress.balance);
//     });
//
//     [testAddress.addrStr.substring(0, testAddress.addrStr.length - 1),
//       testAddress.addrStr.substring(1, testAddress.addrStr.length), 123456789].forEach((ad) => {
//       it('Should return error if Invalid address: Checksum mismatch',
// () => expect(insight.getBalance(ad))
//         .to.be.rejectedWith('Invalid address: Checksum mismatch. Code:1'));
//     });
//
//
//     ['str', false].forEach((ad) => {
//       it(
//         'Should return error if Invalid address: Input string too short',
//         () => {
//           requestStub.rejects(new Error('Invalid address: Input string too short. Code:1'));
//           expect(insight.getBalance(ad))
// .to.be.rejectedWith('Invalid address: Input string too short. Code:1');
//         },
//       );
//     });
//
//     ['0', '0x'].forEach((ad) => {
//       it(
//         'Should return error if blockHeight Invalid address: Non-base58 character',
//         () => {
//           requestStub.rejects(new Error('Invalid address: Non-base58 character. Code:1'));
//           expect(insight.getBalance(ad))
// .to.be.rejectedWith('Invalid address: Non-base58 character. Code:1');
//         },
//       );
//     });
//   });
//
//   describe('#getUTXO', () => {
//     beforeEach(() => {
//       requestStub = sinon.stub(request, 'get');
//       requestStub.rejects(new Error('Invalid address: Checksum mismatch. Code:1'));
//       requestStub
//         .withArgs(`${URI}/addr/${testAddress.addrStr}/utxo`)
//         .returns(new Promise(resolve => resolve(textUtxo)));
//     });
//
//     it('Should return getBalance', async () => {
//       const utxo = await insight.getUTXO(testAddress.addrStr);
//       expect(utxo).to.be.equal(textUtxo);
//     });
//
//     [testAddress.addrStr.substring(0, testAddress.addrStr.length - 1),
//       testAddress.addrStr.substring(1, testAddress.addrStr.length), 123456789].forEach((ad) => {
//       it('Should return error if Invalid address: Checksum mismatch',
// () => expect(insight.getUTXO(ad))
//         .to.be.rejectedWith('Invalid address: Checksum mismatch. Code:1'));
//     });
//
//
//     ['str', false].forEach((ad) => {
//       it(
//         'Should return error if Invalid address: Input string too short',
//         () => {
//           requestStub.rejects(new Error('Invalid address: Input string too short. Code:1'));
//           expect(insight.getUTXO(ad))
// .to.be.rejectedWith('Invalid address: Input string too short. Code:1');
//         },
//       );
//     });
//
//     ['0', '0x'].forEach((ad) => {
//       it(
//         'Should return error if blockHeight Invalid address: Non-base58 character',
//         () => {
//           requestStub.rejects(new Error('Invalid address: Non-base58 character. Code:1'));
//           expect(insight.getUTXO(ad))
// .to.be.rejectedWith('Invalid address: Non-base58 character. Code:1');
//         },
//       );
//     });
//   });
//
//   describe('#getBestBlockHeight', () => {
//     beforeEach(() => {
//       requestStub = sinon.stub(request, 'get');
//       requestStub.rejects(new Error('not defined'));
//       requestStub
//         .withArgs(`${URI}/status`)
//         .returns(new Promise(resolve => resolve(testStatus)));
//     });
//
//
//     it('Should return getBalance', async () => {
//       const status = await insight.getBestBlockHeight();
//       expect(status).to.be.equal(testStatus.info.blocks);
//     });
//   });
//
//   describe('#getHashFromHeight', () => {
//     beforeEach(() => {
//       requestStub = sinon.stub(request, 'get');
//       requestStub.rejects(new Error('Invalid address: Checksum mismatch. Code:1'));
//       requestStub
//         .withArgs(`${URI}/block-index/${testBlockHeight}`)
//         .returns(new Promise(resolve => resolve(testHashFromHeight)));
//     });
//
//     it('Should return getBalance', async () => {
//       const hashFromHeight = await insight.getHashFromHeight(testBlockHeight);
//       expect(hashFromHeight).to.be.equal(testHashFromHeight.blockHash);
//     });
//
//     // https://dashpay.atlassian.net/browse/EV-950
//     // we can set any value like:
//     //42c56c8ec8e2cdf97a56bf6290c43811ff59181a184b70ebbe3cb66b2970ced2
//     // and results for 42 will be set
//     [testAddress.addrStr.substring(0, testAddress.addrStr.length - 1),
//       testAddress.addrStr.substring(1, testAddress.addrStr.length), 123456789].forEach((ad) => {
//       it('Should return error if Invalid address: Checksum mismatch',
// () => expect(insight.getHashFromHeight(ad))
//         .to.be.rejectedWith('Invalid address: Checksum mismatch. Code:1'));
//     });
//
//     [23498217349279, false, 'fakjfndkjlanmfdas', '0x'].forEach((ad) => {
//       it(
//         'Should return error if JSON value is not an integer as expected. Code:-1',
//         () => {
//           requestStub.rejects(new Error('JSON value is not an integer as expected. Code:-1'));
//           expect(insight.getHashFromHeight(ad))
// .to.be.rejectedWith('JSON value is not an integer as expected. Code:-1');
//         },
//       );
//     });
//
//     ['1396bc62950f8b3a3811f1134b64816273c72d19d9641d747cfd55bea9e120bd'].forEach((ad) => {
//       it(
//         'Should return error if Block height out of range. Code:-8',
//         () => {
//           requestStub.rejects(new Error('Block height out of range. Code:-8'));
//           expect(insight.getHashFromHeight(ad))
// .to.be.rejectedWith('Block height out of range. Code:-8');
//         },
//       );
//     });
//   });
//
//   describe('#getHistoricBlockchainDataSyncStatus', () => {
//     beforeEach(() => {
//       requestStub = sinon.stub(request, 'get');
//       requestStub.rejects(new Error('not defined'));
//       requestStub
//         .withArgs(`${URI}/sync`)
//         .returns(new Promise(resolve => resolve(testSync)));
//     });
//
//
//     it('Should return getBalance', async () => {
//       const sync = await insight.getHistoricBlockchainDataSyncStatus();
//       expect(sync).to.be.equal(testSync);
//     });
//   });
//
//   describe('#getStatus', () => {
//     beforeEach(() => {
//       requestStub = sinon.stub(request, 'get');
//       requestStub.rejects(new Error('not defined'));
//       requestStub
//         .withArgs(`${URI}/status?q=getInfo`)
//         .returns(new Promise(resolve => resolve(testStatus.info.blocks)));
//       requestStub
//         .withArgs(`${URI}/status?q=getDifficulty`).returns(new Promise(resolve => resolve({
// difficulty: testStatus.info.blocks.info.difficulty })));
//       requestStub
//         .withArgs(`${URI}/status?q=getBestBlockHash`).returns(new Promise(resolve => resolve({
// bestblockhash: testHashFromHeight.blockHash.blockHash })));
//       // TODO: https://dashpay.atlassian.net/browse/EV-941
//       requestStub
//         .withArgs(`${URI}/status?q=getLastBlockHash`)
//         .returns(new Promise(resolve => resolve({})));
//     });
//
//     it('Should return status?q=getInfo', async () => {
//       const status = await insight.getStatus('getInfo');
//       expect(status).to.be.equal(testStatus.info.blocks);
//     });
//
//     it('Should return status?q=getDifficulty', async () => {
//       const status = await insight.getStatus('getDifficulty');
//       expect(status).to.be.deep.equal({ difficulty: testStatus.info.blocks.info.difficulty });
//     });
//
//     it('Should return status?q=getBestBlockHash', async () => {
//       const status = await insight.getStatus('getBestBlockHash');
//       expect(status).to.be.deep.equal({ bestblockhash: testHashFromHeight.blockHash.blockHash });
//     });
//
//     it('Should return status?q=getLastBlockHash', async () => {
//       const status = await insight.getStatus('getLastBlockHash');
//       expect(status).to.be.deep.equal({});
//     });
//
//     // TODO: automate https://dashpay.atlassian.net/browse/EV-942
//     ['fake', 123].forEach((type) => {
//       it(
//         'Should return error if Invalid query string.',
//         () => {
//           requestStub.rejects(new Error('Invalid query string.'));
//           expect(insight.getStatus(type)).to.be.rejectedWith('Invalid query string.');
//         },
//       );
//     });
//   });
//
//   describe('#getPeerDataSyncStatus', () => {
//     beforeEach(() => {
//       requestStub = sinon.stub(request, 'get');
//       requestStub.rejects(new Error('not defined'));
//       requestStub
//         .withArgs(`${URI}/peer`)
//         .returns(new Promise(resolve => resolve(testPeer)));
//     });
//
//
//     it('Should return getBalance', async () => {
//       const peer = await insight.getPeerDataSyncStatus();
//       expect(peer).to.be.equal(testPeer);
//     });
//   });
//
//   describe('#estimateFee', () => {
//     beforeEach(() => {
//       requestStub = sinon.stub(request, 'get');
//       requestStub.rejects(new Error('Expected type number, got null. Code:-3'));
//       requestStub
//         .withArgs(`${URI}/utils/estimatefee`)
//         .returns(new Promise(resolve => resolve({
//           2: -1,
//         })));
//       requestStub
//         .withArgs(`${URI}/utils/estimatefee?nbBlocks=1000`)
//         .returns(new Promise(resolve => resolve({
//           1000: -1,
//         })));
//     });
//
//     it('Should return estimateFee', async () => {
//       const estimateFee = await insight.estimateFee();
//       expect(estimateFee).to.be.deep.equal({
//         2: -1,
//       });
//     });
//
//     it('Should return estimateFee', async () => {
//       const estimateFee = await insight.estimateFee(1000);
//       expect(estimateFee).to.be.deep.equal({
//         1000: -1,
//       });
//     });
//
//     [testAddress.addrStr.substring(0, testAddress.addrStr.length - 1),
//       testAddress.addrStr.substring(1, testAddress.addrStr.length), 123456789].forEach((ad) => {
//       it('Should return error if Invalid address: Checksum mismatch',
// () => expect(insight.estimateFee(ad))
//         .to.be.rejectedWith('Expected type number, got null. Code:-3'));
//     });
//
//
//     [false, 'fakjfndkjlanmfdas'].forEach((ad) => {
//       it(
//         'Should return error if JSON value is not an integer as expected. Code:-1',
//         () => {
//           requestStub.rejects(new Error('JSON value is not an integer as expected. Code:-1'));
//           expect(insight.estimateFee(ad))
// .to.be.rejectedWith('JSON value is not an integer as expected. Code:-1');
//         },
//       );
//     });
//
//     [3425245234523453].forEach((ad) => {
//       it(
//         'Should return error if JSON integer out of range.',
//         () => {
//           requestStub.rejects(new Error('JSON integer out of range. Code:-1'));
//           expect(insight.estimateFee(ad))
// .to.be.rejectedWith('JSON integer out of range. Code:-1');
//         },
//       );
//     });
//   });
//
//   describe('#getBlockHeaders', () => {
//     beforeEach(() => {
//       requestStub = sinon.stub(request, 'get');
//       requestStub.rejects(new Error('Block not found. Code:-5'));
//       requestStub
//         .withArgs(`${URI}/block-headers/1/1`)
//         .returns(new Promise(resolve => resolve({ headers: [testBlockHeaders.headers[0]] })));
//       requestStub
//         .withArgs(`${URI}/block-headers/1/2`)
//         .returns(new Promise(resolve => resolve(testBlockHeaders)));
//     });
//
//     it('Should return getBlockHeaders', async () => {
//       const estimateFee = await insight.getBlockHeaders(1, 1);
//       expect(estimateFee).to.be.deep.equal({ headers: [testBlockHeaders.headers[0]] });
//     });
//
//     it('Should return getBlockHeaders', async () => {
//       const estimateFee = await insight.getBlockHeaders(1, 2);
//       expect(estimateFee).to.be.deep.equal(testBlockHeaders);
//     });
//
//     [false, 'fakjfndkjlanmfdas'].forEach((ad) => {
//       it('Should return error if Invalid address: Checksum mismatch',
// () => expect(insight.getBlockHeaders(ad))
//         .to.be.rejectedWith('Block not found. Code:-5'));
//     });
//
//     [2313123213123332134].forEach((ad) => {
//       it(
//         'Should return error if JSON integer out of range',
//         () => {
//           requestStub.rejects(new Error('JSON integer out of range. Code:-1'));
//           expect(insight.getBlockHeaders(ad))
// .to.be.rejectedWith('JSON integer out of range. Code:-1');
//         },
//       );
//     });
//   });
//
//   describe('#getMasternodesList', () => {
//     beforeEach(() => {
//       requestStub = sinon.stub(request, 'get');
//       requestStub.rejects(new Error('not defined'));
//       requestStub
//         .withArgs(`${URI}/masternodes/list`)
//         .returns(new Promise(resolve => resolve([])));
//     });
//
//     it('Should return getMasternodesList', async () => {
//       const status = await insight.getMasternodesList();
//       expect(status).to.be.deep.equal([]);
//     });
//   });
//
//   describe('#getMnList & getMnUpdateList', () => {
//     beforeEach(() => {
//       requestStub = sinon.stub(request, 'get');
//       requestStub.rejects(new Error('not defined'));
//       requestStub
//         .withArgs(`${URI}/masternodes/list`)
//         .returns(new Promise(resolve => resolve([])));
//     });
//
//     it('Should return getMnList', async () => {
//       const getMnList = await insight.getMnList();
//       expect(getMnList).to.have.lengthOf(40);
//     });
//
//     it('Should return getMnUpdateList', async () => {
//       const getMnUpdateList = await insight.getMnUpdateList();
//       expect(getMnUpdateList.type).to.be.equals('full');
//       expect(getMnUpdateList.list).to.have.lengthOf(40);
//     });
//   });
// });
