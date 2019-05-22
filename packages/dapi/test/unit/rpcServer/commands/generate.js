const chai = require('chai');
const sinon = require('sinon');
const chaiAsPromised = require('chai-as-promised');
const generateFactory = require('../../../../lib/rpcServer/commands/generate');
const coreAPIFixture = require('../../../mocks/coreAPIFixture');

const { expect } = chai;
chai.use(chaiAsPromised);
let spy;

describe('generate', () => {
  describe('#factory', () => {
    it('should return a function', () => {
      const generate = generateFactory(coreAPIFixture);
      expect(generate).to.be.a('function');
    });
  });

  before(() => {
    spy = sinon.spy(coreAPIFixture, 'generate');
  });

  beforeEach(() => {
    spy.resetHistory();
  });

  after(() => {
    spy.restore();
  });

  it('Should return an array of block hashes', async () => {
    const generate = generateFactory(coreAPIFixture);
    expect(spy.callCount).to.be.equal(0);
    const blockHashes = await generate({ amount: 10 });
    expect(blockHashes).to.be.an('array');
    expect(blockHashes.length).to.be.equal(10);
    expect(spy.callCount).to.be.equal(1);
  });

  it('Should throw an error if arguments are not valid', async () => {
    const generate = generateFactory(coreAPIFixture);
    expect(spy.callCount).to.be.equal(0);
    await expect(generate({ amount: -1 })).to.be.rejectedWith('should be >= 0');
    expect(spy.callCount).to.be.equal(0);
    await expect(generate({ amount: 0.5 })).to.be.rejectedWith('should be integer');
    expect(spy.callCount).to.be.equal(0);
    await expect(generate({})).to.be.rejectedWith('should have required property');
    expect(spy.callCount).to.be.equal(0);
    await expect(generate()).to.be.rejectedWith('should be object');
    expect(spy.callCount).to.be.equal(0);
    await expect(generate({ amount: 'string' })).to.be.rejectedWith('should be integer');
    expect(spy.callCount).to.be.equal(0);
    await expect(generate([-1])).to.be.rejected;
    expect(spy.callCount).to.be.equal(0);
  });
});
