const {expect} = require('chai');
const ensureAddressesToGapLimit = require('./ensureAddressesToGapLimit');
const {CONSTANTS} = require("../../index");

const walletStore = {
  addresses: {
    external: {},
    internal: {}
  }
}

const walletType = CONSTANTS.WALLET_TYPES.HDWALLET;
const accountIndex = 0;
const getAddress = (i, type) => {
  const rootPath = `m/44'/1'/0'`;
  const path = `${rootPath}/${(type === 'external') ? `0` : '1'}/${i}`;
  if (!walletStore.addresses[type][path]) {
    walletStore.addresses[type][path] = {
      index: i,
      path,
      used: false
    }
  }
  return walletStore.addresses[type][path];
}
describe('Utils - BIP44 - ensureAddressesToGapLimit', function suite() {

  it('should set first set of 20 unused address in a row', function () {
    const generated = ensureAddressesToGapLimit(walletStore, walletType, accountIndex, getAddress)
    expect(generated).to.equal(40);
    expect(Object.keys(walletStore.addresses.external))
        .to.deep.equal(["m/44'/1'/0'/0/0", "m/44'/1'/0'/0/1", "m/44'/1'/0'/0/2", "m/44'/1'/0'/0/3", "m/44'/1'/0'/0/4", "m/44'/1'/0'/0/5", "m/44'/1'/0'/0/6", "m/44'/1'/0'/0/7", "m/44'/1'/0'/0/8", "m/44'/1'/0'/0/9", "m/44'/1'/0'/0/10", "m/44'/1'/0'/0/11", "m/44'/1'/0'/0/12", "m/44'/1'/0'/0/13", "m/44'/1'/0'/0/14", "m/44'/1'/0'/0/15", "m/44'/1'/0'/0/16", "m/44'/1'/0'/0/17", "m/44'/1'/0'/0/18", "m/44'/1'/0'/0/19"]);
    expect(Object.keys(walletStore.addresses.internal))
        .to.deep.equal(["m/44'/1'/0'/1/0", "m/44'/1'/0'/1/1", "m/44'/1'/0'/1/2", "m/44'/1'/0'/1/3", "m/44'/1'/0'/1/4", "m/44'/1'/0'/1/5", "m/44'/1'/0'/1/6", "m/44'/1'/0'/1/7", "m/44'/1'/0'/1/8", "m/44'/1'/0'/1/9", "m/44'/1'/0'/1/10", "m/44'/1'/0'/1/11", "m/44'/1'/0'/1/12", "m/44'/1'/0'/1/13", "m/44'/1'/0'/1/14", "m/44'/1'/0'/1/15", "m/44'/1'/0'/1/16", "m/44'/1'/0'/1/17", "m/44'/1'/0'/1/18", "m/44'/1'/0'/1/19"])
  });
  it('should always have a gap of 20 unused address in a row', function () {
    for (let i = 0; i < 10; i++) {
      walletStore.addresses.external[`m/44'/1'/0'/0/${i}`].used = true
      walletStore.addresses.internal[`m/44'/1'/0'/1/${i}`].used = true
    }

    const generated = ensureAddressesToGapLimit(walletStore, walletType, accountIndex, getAddress)
    expect(generated).to.equal(20);
    expect(Object.keys(walletStore.addresses.external))
        .to.deep.equal(["m/44'/1'/0'/0/0", "m/44'/1'/0'/0/1", "m/44'/1'/0'/0/2", "m/44'/1'/0'/0/3", "m/44'/1'/0'/0/4", "m/44'/1'/0'/0/5", "m/44'/1'/0'/0/6", "m/44'/1'/0'/0/7", "m/44'/1'/0'/0/8", "m/44'/1'/0'/0/9", "m/44'/1'/0'/0/10", "m/44'/1'/0'/0/11", "m/44'/1'/0'/0/12", "m/44'/1'/0'/0/13", "m/44'/1'/0'/0/14", "m/44'/1'/0'/0/15", "m/44'/1'/0'/0/16", "m/44'/1'/0'/0/17", "m/44'/1'/0'/0/18", "m/44'/1'/0'/0/19", "m/44'/1'/0'/0/20", "m/44'/1'/0'/0/21", "m/44'/1'/0'/0/22", "m/44'/1'/0'/0/23", "m/44'/1'/0'/0/24", "m/44'/1'/0'/0/25", "m/44'/1'/0'/0/26", "m/44'/1'/0'/0/27", "m/44'/1'/0'/0/28", "m/44'/1'/0'/0/29"]);
    expect(Object.keys(walletStore.addresses.internal))
        .to.deep.equal(["m/44'/1'/0'/1/0", "m/44'/1'/0'/1/1", "m/44'/1'/0'/1/2", "m/44'/1'/0'/1/3", "m/44'/1'/0'/1/4", "m/44'/1'/0'/1/5", "m/44'/1'/0'/1/6", "m/44'/1'/0'/1/7", "m/44'/1'/0'/1/8", "m/44'/1'/0'/1/9", "m/44'/1'/0'/1/10", "m/44'/1'/0'/1/11", "m/44'/1'/0'/1/12", "m/44'/1'/0'/1/13", "m/44'/1'/0'/1/14", "m/44'/1'/0'/1/15", "m/44'/1'/0'/1/16", "m/44'/1'/0'/1/17", "m/44'/1'/0'/1/18", "m/44'/1'/0'/1/19", "m/44'/1'/0'/1/20", "m/44'/1'/0'/1/21", "m/44'/1'/0'/1/22", "m/44'/1'/0'/1/23", "m/44'/1'/0'/1/24", "m/44'/1'/0'/1/25", "m/44'/1'/0'/1/26", "m/44'/1'/0'/1/27", "m/44'/1'/0'/1/28", "m/44'/1'/0'/1/29"]);
  });
  it('should keep gap for each type ', function () {
    for (let i = 0; i < 15; i++) {
      walletStore.addresses.internal[`m/44'/1'/0'/1/${i}`].used = true
    }
    const generated = ensureAddressesToGapLimit(walletStore, walletType, accountIndex, getAddress)
    expect(generated).to.equal(5);
  });

});
