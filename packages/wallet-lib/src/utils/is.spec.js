const { Mnemonic, Networks, Address } = require('@dashevo/dashcore-lib');
const { expect } = require('chai');
const { is, generateNewMnemonic} = require("./index");
const figureBridgeFixture = require("../../fixtures/figurebridge");

describe('Utils - is', function suite() {
  it('should have is.num handle numbers', () => {
    expect(is.num(100)).to.be.equals(true);
  });
  it('should have is.num handle not numbers', () => {
    expect(is.num('100')).to.be.equals(false);
  });
  it('should have is.arr handle empty arr', () => {
    expect(is.arr([])).to.be.equals(true);
  });
  it('should have is.arr handle arr', () => {
    expect(is.arr([1, 'b'])).to.be.equals(true);
  });
  it('should have is.arr handle not array(dict)', () => {
    expect(is.arr({ 100: 'b' })).to.be.equals(false);
  });
  it('should have is.arr handle not array(str)', () => {
    expect(is.arr('str')).to.be.equals(false);
  });
  it('should have is.float handle int', () => {
    expect(is.float(100)).to.be.equals(false);
  });
  it('should have is.float handle float with .0(not float)', () => {
    expect(is.float(100.0)).to.be.equals(false);
  });
  it('should have is.float handle float', () => {
    expect(is.float(100.2)).to.be.equals(true);
  });
  it('should have is.float handle not float(str)', () => {
    expect(is.num('100')).to.be.equals(false);
  });

  it('should have is.int handle int', () => {
    expect(is.int(100)).to.be.equals(true);
  });
  it('should have is.int handle zero', () => {
    expect(is.int(0)).to.be.equals(true);
  });
  it('should have is.int handle negative int', () => {
    expect(is.int(-1)).to.be.equals(true);
  });

  it('should have is.int handle float with .0', () => {
    expect(is.int(100.0)).to.be.equals(true);
  });
  it('should have is.int handle float', () => {
    expect(is.int(100.2)).to.be.equals(false);
  });
  it('should have is.int handle not float(str)', () => {
    expect(is.int('100')).to.be.equals(false);
  });

  it('should have is.bool handle true', () => {
    expect(is.bool(true)).to.be.equals(true);
  });
  it('should have is.bool handle false', () => {
    expect(is.bool(false)).to.be.equals(true);
  });
  it('should have is.bool handle int', () => {
    expect(is.bool('true')).to.be.equals(false);
  });
  it('should have is.hex handle hex', () => {
    expect(is.hex('1234567890ABCD')).to.be.equals(true);
    expect(is.hex('0b757a848f')).to.equal(true);
    expect(is.hex('')).to.be.equals(true);
  });
  it('should have is.hex handle not hex', () => {
    expect(is.hex('12648430T')).to.be.equals(false);
  });

  it('should have is.obj handle obj', () => {
    expect(is.obj(generateNewMnemonic())).to.be.equals(true);
  });
  it('should have is.obj handle primitive value', () => {
    expect(is.obj(false)).to.be.equals(false);
  });
  it('should have is.obj handle array', () => {
    expect(is.obj(['false'])).to.be.equals(true);
  });

  it('should have is.fn handle obj', () => {
    expect(is.fn(generateNewMnemonic)).to.be.equals(true);
  });
  it('should have is.fn handle primitive value', () => {
    expect(is.fn(false)).to.be.equals(false);
  });
  it('should have is.fn handle arrow function', () => {
    expect(is.fn(() => generateNewMnemonic)).to.be.equals(true);
  });

  it('should have is.def handle any value', () => {
    expect(is.def(1)).to.be.equals(true);
  });
  it('should have is.def handle undefined', () => {
    expect(is.def(undefined)).to.be.equals(false);
  });

  it('should have is.undef handle undefined', () => {
    expect(is.undef(undefined)).to.be.equals(true);
  });
  it('should have is.undef handle any value', () => {
    expect(is.undef('undefined')).to.be.equals(false);
  });

  it('should have is.null handle null', () => {
    expect(is.null(null)).to.be.equals(true);
  });
  it('should have is.null handle any value', () => {
    expect(is.null('null')).to.be.equals(false);
  });

  it('should have is.promise handle promise', () => {
    const promise = new Promise((() => {
    }));
    expect(is.promise(promise)).to.be.equals(true);
  });
  it('should have is.promise handle non promise', () => {
    expect(is.promise(() => generateNewMnemonic)).to.be.equals(false);
  });

  it('should have is.JSON handle empty json', () => {
    expect(is.JSON()).to.be.equals(true);
  });
  it('should have is.JSON handle array', () => {
    expect(is.JSON([1, 2])).to.be.equals(true);
  });
  it('should have is.JSON handle str as json', () => {
    expect(is.JSON('str')).to.be.equals(true);
  });
  it('should have is.JSON not allow circular references', () => {
    const circularReference = {};
    circularReference.myself = circularReference;
    expect(is.JSON(circularReference)).to.be.equals(false);
  });

  it('should have is.stringified handle empty JSON', () => {
    expect(is.stringified('{}')).to.be.equals(true);
  });
  it('should have is.stringified handle JSON', () => {
    expect(is.stringified('{"result":true, "count":42}')).to.be.equals(true);
  });
  it('should have is.stringified handle str', () => {
    expect(is.stringified('true')).to.be.equals(true);
  });
  it('should have is.stringified not allow circular references', () => {
    const circularReference = {};
    circularReference.myself = circularReference;
    expect(is.stringified(circularReference)).to.be.equals(false);
  });
  it('should have is.type handle type', () => {
    const arr = [];
    expect(is.type(arr, 'Array')).to.be.equal(true);
  });
  it('should have is.mnemonic work', () => {
    const mnemonic = new Mnemonic();
    const mnemonic2 = 'crack spice venue ticket vacant steak next stomach amateur review okay curtain';
    expect(is.mnemonic(mnemonic)).to.be.equal(true);
    expect(is.mnemonic(mnemonic2)).to.be.equal(true);
  });
  it('should have is.network work', () => {
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
  it('should have is.seed work', () => {
    const seed = new Mnemonic().toSeed();
    const seed2 = new Mnemonic().toHDPrivateKey();
    expect(is.seed(seed.toString('hex'))).to.be.equal(true);
    expect(is.seed(seed2)).to.be.equal(false);
  });
  it('should have is.HDPrivateKey work', () => {
    const seed = new Mnemonic().toSeed();
    const seed2 = new Mnemonic().toHDPrivateKey();
    expect(is.seed(seed.toString('hex'))).to.be.equal(true);
    expect(is.seed(seed2)).to.be.equal(false);
  });
  it('should have is.address work', () => {
    const addr = new Address('yinidcHwrfzb4bEJDSq3wtQyxRAgQxsQia');
    expect(is.address(addr)).to.be.equal(true);
  });
  it('should have is.txid work', () => {
    const validtxid = '56150e17895255d178eb4d3da0ccd580fdf50233a3767e1f562e05f00b48cf79';
    expect(is.txid(validtxid)).to.be.equal(true);

    const invalidtxid = '00000';
    expect(is.txid(invalidtxid)).to.be.equal(false);
  });
  it('should have is.transactionObj work', () => {
    const validTransaction = figureBridgeFixture.transactions['3428f0c29370d1293b4706ffd0f8b0c84a5b7c1c217d319e5ef4722354000c6e'];
    expect(is.transactionObj(validTransaction)).to.be.equal(true);

    const invalidTransaction = {
      vin: [],
      vout: [],
    };
    expect(is.transactionObj(invalidTransaction)).to.be.equal(false);
  });
  it('should have is.feeRate work', () => {
    const feeRate = {
      type: 'perBytes',
      value: 10,
    };
    expect(is.feeRate(feeRate)).to.be.equal(true);
  });

});
