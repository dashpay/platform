// process.on('unhandledRejection', (up) => { throw up; });
// const chai = require('chai');
// const chaiAsPromised = require('chai-as-promised');
// const sinon = require('sinon');
// const KeyValueStore = require('../../../lib/services/mempool/KeyValueStore');
//
//
// chai.use(chaiAsPromised);
// const { expect } = chai;
//
// describe('KeyValueStore', () => {
//   it('should call someFunction()', () => {
//     const spy = sinon.spy(KeyValueStore);
//     const inst = new KeyValueStore(5001);
//   });
//
//   ['port', 22, true, 33333, -1, 65537].forEach((port) => {
//     it('.constictor.invalid port', async () => {
//       const ks = new KeyValueStore(port);
//       await expect(ks.init()).to.be.rejectedWith('KeyValueStore could not be initialized.');
//     });
//   });
//
//   it('.writeValue without initialization', async () => {
//     const ks = new KeyValueStore(5001);
//     expect(() => { ks.writeValue('key', 'value'); })
// .to.throw('KeyValueStore hasn\'t been initialized. Run the init() method first.');
//   });
//
//   it('.getValue without initialization', async () => {
//     const ks = new KeyValueStore(5001);
//     expect(() => { ks.getValue('key'); })
// .to.throw('KeyValueStore hasn\'t been initialized. Run the init() method first.');
//   });
//
//   it('.contains without initialization', async () => {
//     const ks = new KeyValueStore(5001);
//     expect(() => { ks.contains('test'); })
// .to.throw('KeyValueStore hasn\'t been initialized. Run the init() method first');
//   });
//
//   it('error when change hasBeenInitialized by force', async () => {
//     const ks = new KeyValueStore(5551);
//     ks.init('message');
//     ks.hasBeenInitialized = true;
//     expect(() => ks.getValue('no_value')).to.throw('Cannot read property \'get\' of undefined');
//   });
// });
