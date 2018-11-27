const chai = require('chai');
const chaiAsPromised = require('chai-as-promised');
const sinon = require('sinon');
const getUTXOFactory = require('../../../lib/rpcServer/commands/getUTXO.js');
const coreAPIFixture = require('../../fixtures/coreAPIFixture');

const { expect } = chai;
chai.use(chaiAsPromised);
let spy;

describe('getUTXO', () => {
  describe('#factory', () => {
    it('should return a function', () => {
      const getUTXO = getUTXOFactory(coreAPIFixture);
      expect(getUTXO).to.be.a('function');
    });
  });

  before(() => {
    spy = sinon.spy(coreAPIFixture, 'getUTXO');
  });

  beforeEach(() => {
    spy.resetHistory();
  });

  after(() => {
    spy.restore();
  });

  it('Should return an array of unspent outputs', async () => {
    const getUTXO = getUTXOFactory(coreAPIFixture);
    expect(spy.callCount).to.be.equal(0);
    const UTXO = await getUTXO({ address: 'XsLdVrfJpzt6Fc8RSUFkqYqtxkLjEv484w' });
    expect(UTXO).to.be.an('array');
    expect(spy.callCount).to.be.equal(1);
  });

  it('Should throw if arguments are not valid', async () => {
    const getUTXO = getUTXOFactory(coreAPIFixture);
    expect(spy.callCount).to.be.equal(0);
    await expect(getUTXO([])).to.be.rejected;
    expect(spy.callCount).to.be.equal(0);
    await expect(getUTXO({})).to.be.rejectedWith('should have required property \'address\'');
    expect(spy.callCount).to.be.equal(0);
    await expect(getUTXO({ address: 1 })).to.be.rejectedWith('address should be string');
    expect(spy.callCount).to.be.equal(0);
  });
});
