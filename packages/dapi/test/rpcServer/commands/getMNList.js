const chai = require('chai');
const chaiAsPromised = require('chai-as-promised');
const sinon = require('sinon');
const getMNListFactory = require('../../../lib/rpcServer/commands/getMNList.js');
const coreAPIFixture = require('../../fixtures/coreAPIFixture');

chai.use(chaiAsPromised);
const { expect } = chai;
let spy;

describe('getMNList', () => {
  describe('#factory', () => {
    it('should return a function', () => {
      const getMNList = getMNListFactory(coreAPIFixture);
      expect(getMNList).to.be.a('function');
    });
  });

  before(() => {
    spy = sinon.spy(coreAPIFixture, 'getMasternodesList');
  });

  beforeEach(() => {
    spy.resetHistory();
  });

  after(() => {
    spy.restore();
  });

  it('Should return a masternode list', async () => {
    const getMNList = getMNListFactory(coreAPIFixture);
    expect(spy.callCount).to.be.equal(0);
    const MNList = await getMNList();
    expect(MNList).to.be.an('array');
    expect(MNList[0]).to.have.property('ip');
    expect(spy.callCount).to.be.equal(1);
  });
});
