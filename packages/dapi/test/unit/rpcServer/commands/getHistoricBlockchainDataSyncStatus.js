const chai = require('chai');
const chaiAsPromised = require('chai-as-promised');
const sinon = require('sinon');
const getHBDSSFactory = require('../../../../lib/rpcServer/commands/getHistoricBlockchainDataSyncStatus.js');
const coreAPIFixture = require('../../../mocks/coreAPIFixture');

chai.use(chaiAsPromised);
const { expect } = chai;
let spy;

describe('getBestBlockHeight', () => {
  describe('#factory', () => {
    it('should return a function', () => {
      const getHistoricBlockchainDataSyncStatus = getHBDSSFactory(coreAPIFixture);
      expect(getHistoricBlockchainDataSyncStatus).to.be.a('function');
    });
  });

  before(() => {
    spy = sinon.spy(coreAPIFixture, 'getHistoricBlockchainDataSyncStatus');
  });

  beforeEach(() => {
    spy.resetHistory();
  });

  after(() => {
    spy.restore();
  });

  it('Should return an object', async () => {
    const getHistoricBlockchainDataSyncStatus = getHBDSSFactory(coreAPIFixture);
    expect(spy.callCount).to.be.equal(0);
    const bestBlockHeight = await getHistoricBlockchainDataSyncStatus(['XsLdVrfJpzt6Fc8RSUFkqYqtxkLjEv484w']);
    expect(bestBlockHeight).to.be.an('object');
    expect(spy.callCount).to.be.equal(1);
  });
});
