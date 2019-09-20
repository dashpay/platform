const { expect } = require('chai');
const Dashcore = require('@dashevo/dashcore-lib');
const getUnusedAddress = require('../../../src/types/Account/methods/getUnusedAddress');
const getAddress = require('../../../src/types/Account/methods/getAddress');
const generateAddress = require('../../../src/types/Account/methods/generateAddress');
const KeyChain = require('../../../src/types/KeyChain/KeyChain');

const mockedStore = require('../../fixtures/duringdevelop-fullstore-snapshot-1548538361');

const HDRootKeyMockedStore = 'tprv8ZgxMBicQKsPfEan1JB7NF4STbvnjGvP9318CN7FPGZp5nsUTBqmerxtDVpsJjFufyfkTgoe6QfHcDhMqjN3ZoFKtb8SnXFeubNjQreZSq6';

describe('Account - getUnusedAddress', () => {
  it('should get the proper unused address', () => {
    const self = Object.assign({}, {
      store: mockedStore,
      storage: {
        getStore: () => mockedStore,
        importAddresses: _ => (_),
      },
      events: {
        emit: _ => (_),
      },
      keyChain: new KeyChain({
        type: 'HDPrivateKey',
        HDPrivateKey: Dashcore.HDPrivateKey(HDRootKeyMockedStore),
      }),
      BIP44PATH: 'm/44\'/1\'/0\'',
      walletId: '5061b8276c',
      accountIndex: 0,
    });
    self.getAddress = getAddress.bind(self);
    self.generateAddress = generateAddress.bind(self);

    const unusedAddressExternal = getUnusedAddress.call(self);
    const unusedAddressInternal = getUnusedAddress.call(self, 'internal');

    // console.log(mockedStore.wallets[self.walletId].addresses.internal)
    expect(unusedAddressExternal).to.be.deep.equal({
      address: 'yaVrJ5dgELFkYwv6AydDyGPAJQ5kTJXyAN',
      balanceSat: 0,
      fetchedLast: 1548538385006,
      path: 'm/44\'/1\'/0\'/0/5',
      transactions: [],
      unconfirmedBalanceSat: 0,
      utxos: {},
      used: false,
    });
    expect(unusedAddressInternal).to.be.deep.equal({
      address: 'yaZFt1VnAbi72mtyjDNV4AwTECqdg5Bv95',
      balanceSat: 0,
      fetchedLast: 1548538385164,
      path: 'm/44\'/1\'/0\'/1/8',
      transactions: [],
      unconfirmedBalanceSat: 0,
      utxos: {},
      used: false,
    });
  });
});
