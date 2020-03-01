const chai = require('chai');
const sinon = require('sinon');
const chaiAsPromised = require('chai-as-promised');
const generateToAddressFactory = require('../../../../lib/rpcServer/commands/generateToAddress');
const coreAPIFixture = require('../../../mocks/coreAPIFixture');

const { expect } = chai;
chai.use(chaiAsPromised);
let spy;

describe('generateToAddress', () => {
  describe('#factory', () => {
    it('should return a function', () => {
      const generate = generateToAddressFactory(coreAPIFixture);
      expect(generate).to.be.a('function');
    });
  });

  before(() => {
    spy = sinon.spy(coreAPIFixture, 'generateToAddress');
  });

  beforeEach(() => {
    spy.resetHistory();
  });

  after(() => {
    spy.restore();
  });

  it('Should return an array of block hashes', async () => {
    const generateToAddress = generateToAddressFactory(coreAPIFixture);

    expect(spy.callCount).to.be.equal(0);

    const blockHashes = await generateToAddress({ blocksNumber: 10, address: '123456' });

    expect(blockHashes).to.be.an('array');
    expect(blockHashes.length).to.be.equal(10);
    expect(spy.callCount).to.be.equal(1);
  });

  it('Should throw an error if arguments are not valid', async () => {
    const generateToAddress = generateToAddressFactory(coreAPIFixture);
    expect(spy.callCount).to.be.equal(0);

    await expect(generateToAddress({ blocksNumber: -1, address: '123' })).to.be.rejectedWith('should be >= 1');
    expect(spy.callCount).to.be.equal(0);

    await expect(generateToAddress({ blocksNumber: 0.5, address: '123' })).to.be.rejectedWith('should be integer');
    expect(spy.callCount).to.be.equal(0);

    await expect(generateToAddress({})).to.be.rejectedWith('should have required property');
    expect(spy.callCount).to.be.equal(0);

    await expect(generateToAddress()).to.be.rejectedWith('should be object');
    expect(spy.callCount).to.be.equal(0);

    await expect(generateToAddress({ blocksNumber: 'string' })).to.be.rejectedWith('should be integer');
    expect(spy.callCount).to.be.equal(0);

    await expect(generateToAddress({ blocksNumber: 1, address: 1 })).to.be.rejectedWith('should be string');
    expect(spy.callCount).to.be.equal(0);
  });
});
