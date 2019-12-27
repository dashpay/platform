const { expect } = require('chai');
const addNewTxToAddress = require('../../../../src/types/Storage/methods/addNewTxToAddress');
// const { fd7c727155ef67fd5c1d54b73dea869e9690c439570063d6e96fec1d3bba450e } = require('../fixtures/transactions').valid;

// FIXME : We only use this method in one specific case : From the SyncWorker when receiving from the WSock from insight
// THerefore we might want to remove our dependency on this method ?
describe('Storage - addNewTxToAddress', () => {
  it('should add a new transaction to an address in store', () => {
    // const self = {
    //   store: {
    //     wallets: {
    //       '123ae': {
    //         addresses: {
    //           internal: {},
    //           external: {},
    //           misc: {},
    //         },
    //         transactions:{
    //
    //         }
    //       },
    //     },
    //   },
    // };
    //
    // addNewTxToAddress.call(self, fd7c727155ef67fd5c1d54b73dea869e9690c439570063d6e96fec1d3bba450e, '123ae');
  });
});
