const chai = require('chai');
const chaiAsPromised = require('chai-as-promised');
const sinon = require('sinon');
const getPeerDataSyncStatusFactory = require('../../../lib/rpcServer/commands/getPeerDataSyncStatus');
const coreAPIFixture = require('../../fixtures/coreAPIFixture');

chai.use(chaiAsPromised);
const { expect } = chai;
let spy;

describe('getMNList', () => {
  describe('#factory', () => {
    it('should return a function', () => {
      const getMNList = getPeerDataSyncStatusFactory(coreAPIFixture);
      expect(getMNList).to.be.a('function');
    });
  });

  before(() => {
    spy = sinon.spy(coreAPIFixture, 'getPeerDataSyncStatus');
  });

  beforeEach(() => {
    spy.resetHistory();
  });

  after(() => {
    spy.restore();
  });

  it('should return a peerDataSyncStatus list', async () => {
    const getPeerDataSyncStatus = getPeerDataSyncStatusFactory(coreAPIFixture);
    expect(spy.callCount).to.be.equal(0);
    const peerDataSyncStatus = await getPeerDataSyncStatus();
    // TODO: is peerDataSyncStatus really supposed to return an empty string?
    expect(peerDataSyncStatus).to.be.equal('');
    expect(spy.callCount).to.be.equal(1);
  });
});
