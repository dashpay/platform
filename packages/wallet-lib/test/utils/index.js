const { expect } = require('chai');
const { Networks, Address, Mnemonic } = require('@dashevo/dashcore-lib');
const {
  dashToDuffs,
  duffsToDash,
  generateNewMnemonic,
  mnemonicToHDPrivateKey,
  is,
  hasProp,
  getBytesOf,
} = require('../../src/utils/index');
const knifeEasilyFixture = require('../fixtures/knifeeasily');
const figureBridgeFixture = require('../fixtures/figurebridge');

describe('Utils', () => {
  it('should handle dash2Duff', () => {
    expect(dashToDuffs(2000)).to.equal(200000000000);
    expect(dashToDuffs(-2000)).to.equal(-200000000000);
    expect(() => dashToDuffs('deuxmille')).to.throw('Can only convert a number');
  });
  it('should handle duff2Dash', () => {
    expect(duffsToDash(200000000000)).to.equal(2000);
    expect(duffsToDash(-200000000000)).to.equal(-2000);
    expect(() => duffsToDash('deuxmille')).to.throw('Can only convert a number');
  });
  it('should generate a mnemonic', () => {
    const mnemonic = generateNewMnemonic();
    expect(mnemonic).to.be.a('object');
    expect(mnemonic.toString()).to.be.a('string');
  });
  it('should convert mnemonic to seed', () => {
    const network = Networks.testnet;
    const seed = mnemonicToHDPrivateKey(knifeEasilyFixture.mnemonic, network);
    expect(seed).to.be.a('object');
    expect(seed.toString()).to.equal(knifeEasilyFixture.HDRootPrivateKeyTestnet);
  });
  it('should throw error when mnemonic is not provided in mnemonicToHDPrivateKey', () => {
    const network = Networks.testnet;
    expect(() => mnemonicToHDPrivateKey('', network)).to.throw('Expect mnemonic to be provide');
  });
  it('should is.num handle numbers', () => {
    expect(is.num(100)).to.be.equals(true);
  });
  it('should is.num handle not numbers', () => {
    expect(is.num('100')).to.be.equals(false);
  });
  it('should is.arr handle empty arr', () => {
    expect(is.arr([])).to.be.equals(true);
  });
  it('should is.arr handle arr', () => {
    expect(is.arr([1, 'b'])).to.be.equals(true);
  });
  it('should is.arr handle not array(dict)', () => {
    expect(is.arr({ 100: 'b' })).to.be.equals(false);
  });
  it('should is.arr handle not array(str)', () => {
    expect(is.arr('str')).to.be.equals(false);
  });
  it('should is.float handle int', () => {
    expect(is.float(100)).to.be.equals(false);
  });
  it('should is.float handle float with .0(not float)', () => {
    expect(is.float(100.0)).to.be.equals(false);
  });
  it('should is.float handle float', () => {
    expect(is.float(100.2)).to.be.equals(true);
  });
  it('should is.float handle not float(str)', () => {
    expect(is.num('100')).to.be.equals(false);
  });

  it('should is.int handle int', () => {
    expect(is.int(100)).to.be.equals(true);
  });
  it('should is.int handle zero', () => {
    expect(is.int(0)).to.be.equals(true);
  });
  it('should is.int handle negative int', () => {
    expect(is.int(-1)).to.be.equals(true);
  });

  it('should is.int handle float with .0', () => {
    expect(is.int(100.0)).to.be.equals(true);
  });
  it('should is.int handle float', () => {
    expect(is.int(100.2)).to.be.equals(false);
  });
  it('should is.int handle not float(str)', () => {
    expect(is.int('100')).to.be.equals(false);
  });

  it('should is.bool handle true', () => {
    expect(is.bool(true)).to.be.equals(true);
  });
  it('should is.bool handle false', () => {
    expect(is.bool(false)).to.be.equals(true);
  });
  it('should is.bool handle int', () => {
    expect(is.bool('true')).to.be.equals(false);
  });
  it('should is.hex handle hex', () => {
    expect(is.hex('1234567890ABCD')).to.be.equals(true);
    expect(is.hex('0b757a848f')).to.equal(true);
    expect(is.hex('')).to.be.equals(true);
  });
  it('should is.hex handle not hex', () => {
    expect(is.hex('12648430T')).to.be.equals(false);
  });

  it('should is.obj handle obj', () => {
    expect(is.obj(generateNewMnemonic())).to.be.equals(true);
  });
  it('should is.obj handle primitive value', () => {
    expect(is.obj(false)).to.be.equals(false);
  });
  it('should is.obj handle array', () => {
    expect(is.obj(['false'])).to.be.equals(true);
  });

  it('should is.fn handle obj', () => {
    expect(is.fn(generateNewMnemonic)).to.be.equals(true);
  });
  it('should is.fn handle primitive value', () => {
    expect(is.fn(false)).to.be.equals(false);
  });
  it('should is.fn handle arrow function', () => {
    expect(is.fn(() => generateNewMnemonic)).to.be.equals(true);
  });

  it('should is.def handle any value', () => {
    expect(is.def(1)).to.be.equals(true);
  });
  it('should is.def handle undefined', () => {
    expect(is.def(undefined)).to.be.equals(false);
  });

  it('should is.undef handle undefined', () => {
    expect(is.undef(undefined)).to.be.equals(true);
  });
  it('should is.undef handle any value', () => {
    expect(is.undef('undefined')).to.be.equals(false);
  });

  it('should is.null handle null', () => {
    expect(is.null(null)).to.be.equals(true);
  });
  it('should is.null handle any value', () => {
    expect(is.null('null')).to.be.equals(false);
  });

  it('should is.promise handle promise', () => {
    const promise = new Promise((() => {
    }));
    expect(is.promise(promise)).to.be.equals(true);
  });
  it('should is.promise handle non promise', () => {
    expect(is.promise(() => generateNewMnemonic)).to.be.equals(false);
  });

  it('should is.JSON handle empty json', () => {
    expect(is.JSON()).to.be.equals(true);
  });
  it('should is.JSON handle array', () => {
    expect(is.JSON([1, 2])).to.be.equals(true);
  });
  it('should is.JSON handle str as json', () => {
    expect(is.JSON('str')).to.be.equals(true);
  });
  it('should is.JSON not allow circular references', () => {
    const circularReference = {};
    circularReference.myself = circularReference;
    expect(is.JSON(circularReference)).to.be.equals(false);
  });

  it('should is.stringified handle empty JSON', () => {
    expect(is.stringified('{}')).to.be.equals(true);
  });
  it('should is.stringified handle JSON', () => {
    expect(is.stringified('{"result":true, "count":42}')).to.be.equals(true);
  });
  it('should is.stringified handle str', () => {
    expect(is.stringified('true')).to.be.equals(true);
  });
  it('should is.stringified not allow circular references', () => {
    const circularReference = {};
    circularReference.myself = circularReference;
    expect(is.stringified(circularReference)).to.be.equals(false);
  });
  it('should is.type handle type', () => {
    const arr = [];
    expect(is.type(arr, 'Array')).to.be.equal(true);
  });
  it('should is.mnemonic work', () => {
    const mnemonic = new Mnemonic();
    const mnemonic2 = 'crack spice venue ticket vacant steak next stomach amateur review okay curtain';
    expect(is.mnemonic(mnemonic)).to.be.equal(true);
    expect(is.mnemonic(mnemonic2)).to.be.equal(true);
  });
  it('should is.network work', () => {
    const notANetwork = [];
    const network = Networks.livenet.toString();
    const networktestnet = Networks.testnet.toString();
    const network2 = 'livenet';
    const network2testnet = 'testnet';
    const notanetwork = notANetwork;
    expect(is.network(network)).to.be.equal(true);
    expect(is.network(networktestnet)).to.be.equal(true);
    expect(is.network(network2)).to.be.equal(true);
    expect(is.network(network2testnet)).to.be.equal(true);
    expect(is.network(notanetwork)).to.be.equal(false);
  });
  it('should is.seed work', () => {
    const seed = new Mnemonic().toSeed();
    const seed2 = new Mnemonic().toHDPrivateKey();
    expect(is.seed(seed.toString('hex'))).to.be.equal(true);
    expect(is.seed(seed2)).to.be.equal(false);
  });
  it('should is.HDPrivateKey work', () => {
    const seed = new Mnemonic().toSeed();
    const seed2 = new Mnemonic().toHDPrivateKey();
    expect(is.seed(seed.toString('hex'))).to.be.equal(true);
    expect(is.seed(seed2)).to.be.equal(false);
  });
  it('should is.address work', () => {
    const addr = new Address('yinidcHwrfzb4bEJDSq3wtQyxRAgQxsQia');
    expect(is.address(addr)).to.be.equal(true);
  });
  it('should is.txid work', () => {
    const validtxid = '56150e17895255d178eb4d3da0ccd580fdf50233a3767e1f562e05f00b48cf79';
    expect(is.txid(validtxid)).to.be.equal(true);

    const invalidtxid = '00000';
    expect(is.txid(invalidtxid)).to.be.equal(false);
  });
  it('should is.transactionObj work', () => {
    const validTransaction = figureBridgeFixture.transactions['3428f0c29370d1293b4706ffd0f8b0c84a5b7c1c217d319e5ef4722354000c6e'];
    expect(is.transactionObj(validTransaction)).to.be.equal(true);

    const invalidTransaction = {
      vin: [],
      vout: [],
    };
    expect(is.transactionObj(invalidTransaction)).to.be.equal(false);
  });
  it('should is.feeRate work', () => {
    const feeRate = {
      type: 'perBytes',
      value: 10,
    };
    expect(is.feeRate(feeRate)).to.be.equal(true);
  });
  it('should getBytesOf return false on unknown type', () => {
    expect(getBytesOf(null, 'toto')).to.be.equal(false);
  });
  it('should handle hasProp', () => {
    expect(hasProp({ key1: true }, 'key1')).to.equal(true);
    expect(hasProp({ key1: true }, 'key2')).to.equal(false);
    expect(hasProp(['key1'], 'key1')).to.equal(true);
    expect(hasProp(['key1'], 'key2')).to.equal(false);
    expect(hasProp(null, 'key2')).to.equal(false);
  });
});
