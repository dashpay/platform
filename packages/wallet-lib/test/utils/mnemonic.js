const { expect } = require('chai');
const Mnemonic = require('@dashevo/dashcore-mnemonic');
const Dashcore = require('@dashevo/dashcore-lib');
const {
  generateNewMnemonic, mnemonicToHDPrivateKey, mnemonicToWalletId,
} = require('../../src/utils/mnemonic');
const is = require('../../src/utils/is');

const mnemonic1 = 'hole lesson insane entire dolphin scissors game dwarf polar ethics drip math';
const mnemonic2 = 'woman forest output essay bleak satisfy era ordinary exotic source portion wire';
const mnemonic3 = 'divorce radar castle wire sun timber master income exchange wash fluid loud';
const mnemonic4 = 'increase table banana fiscal innocent wool sport mercy motion stable prize promote';

describe('Utils - mnemonic', () => {
  it('should generate new mnemonic', () => {
    const result = generateNewMnemonic();
    expect(result.constructor.name).to.be.equal('Mnemonic');
  });
  it('should do mnemonicToHDPrivateKey', () => {
    const mnem1 = generateNewMnemonic();
    const mnem2 = generateNewMnemonic().toString();
    const result = mnemonicToHDPrivateKey(mnem1);
    const result2 = mnemonicToHDPrivateKey(mnem2);
    expect(result.constructor.name).to.be.equal('HDPrivateKey');
    expect(result2.constructor.name).to.be.equal('HDPrivateKey');
  });
  it('should do mnemonicToWalletId', () => {
    const mnem1 = generateNewMnemonic();
    const result = mnemonicToWalletId(mnem1);
    expect(result.constructor.name).to.be.equal('String');
    expect(result.length).to.be.equal(10);
    expect(is.hex(result)).to.be.equal(true);

    expect(mnemonicToWalletId(mnemonic1)).to.equal('f566600d81');
    expect(mnemonicToWalletId(mnemonic2)).to.equal('74bbe91a47');
    expect(mnemonicToWalletId(mnemonic3)).to.equal('f351a836e6');
    expect(mnemonicToWalletId(mnemonic4)).to.equal('fad183cbf7');

    expect(() => mnemonicToWalletId()).to.throw('Expect mnemonic to be provided');
    expect(() => mnemonicToHDPrivateKey()).to.throw('Expect mnemonic to be provided');
  });
});
