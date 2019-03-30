const chai = require('chai');
const chaiAsPromised = require('chai-as-promised');
const sinon = require('sinon');
const getContractFactory = require('../../../../lib/rpcServer/commands/fetchContract');
const driveFixture = require('../../../mocks/driveFixture');

chai.use(chaiAsPromised);
const { expect } = chai;
let spy;

describe('fetchContract', () => {
  describe('#factory', () => {
    it('should return a function', () => {
      const getContract = getContractFactory(driveFixture);
      expect(getContract).to.be.a('function');
    });
  });

  before(() => {
    spy = sinon.spy(driveFixture, 'fetchContract');
  });

  beforeEach(() => {
    spy.resetHistory();
  });

  after(() => {
    spy.restore();
  });

  it('Should return search results', async () => {
    const getContract = getContractFactory(driveFixture);
    expect(spy.callCount).to.be.equal(0);
    const contract = await getContract({ contractId: '123' });
    expect(contract).to.be.an('object');
    expect(spy.callCount).to.be.equal(1);
  });

  it('Should throw an error if arguments are not valid', async () => {
    const getContract = getContractFactory(driveFixture);
    expect(spy.callCount).to.be.equal(0);
    await expect(getContract({ contractId: 123 })).to.be.rejectedWith('should be string');
    expect(spy.callCount).to.be.equal(0);
    await expect(getContract({})).to.be.rejectedWith('should have required property');
    expect(spy.callCount).to.be.equal(0);
    await expect(getContract()).to.be.rejectedWith('should be object');
    expect(spy.callCount).to.be.equal(0);
    await expect(getContract([123])).to.be.rejected;
    expect(spy.callCount).to.be.equal(0);
    await expect(getContract([-1])).to.be.rejected;
    expect(spy.callCount).to.be.equal(0);
  });
});
