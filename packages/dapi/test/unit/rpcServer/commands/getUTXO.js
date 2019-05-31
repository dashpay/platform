const chai = require('chai');
const chaiAsPromised = require('chai-as-promised');
const sinon = require('sinon');
const getUTXOFactory = require('../../../../lib/rpcServer/commands/getUTXO.js');
const coreAPIFixture = require('../../../mocks/coreAPIFixture');

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

  it('Should return accept array as input', async () => {
    const getUTXO = getUTXOFactory(coreAPIFixture);
    expect(spy.callCount).to.be.equal(0);
    const addressArray = ['XsLdVrfJpzt6Fc8RSUFkqYqtxkLjEv484w', 'yYmrsYP3XYMZr1cGtha3QzmuNB1C7CfyhV'];
    const UTXO = await getUTXO({ address: addressArray });
    expect(UTXO).to.be.an('array');
    expect(spy.callCount).to.be.equal(1);
  });

  it('Should from-to range be equal to 1000', async () => {
    const getUTXO = getUTXOFactory(coreAPIFixture);
    expect(spy.callCount).to.be.equal(0);
    const addressArray = ['XsLdVrfJpzt6Fc8RSUFkqYqtxkLjEv484w'];
    const UTXO = await getUTXO({ address: addressArray, from: 1, to: 1001 });
    expect(UTXO).to.be.an('array');
    expect(spy.callCount).to.be.equal(1);
  });

  it('Should from-to range be less than 1000', async () => {
    const getUTXO = getUTXOFactory(coreAPIFixture);
    expect(spy.callCount).to.be.equal(0);
    const addressArray = ['XsLdVrfJpzt6Fc8RSUFkqYqtxkLjEv484w'];
    await expect(getUTXO({ address: addressArray, from: 0, to: 1001 })).to.be.rejectedWith('"from" (0) and "to" (1001) range should be less than or equal to 1000');
    expect(spy.callCount).to.be.equal(0);
  });

  it('Should throw if arguments are not valid', async () => {
    const getUTXO = getUTXOFactory(coreAPIFixture);
    expect(spy.callCount).to.be.equal(0);
    await expect(getUTXO([])).to.be.rejected;
    expect(spy.callCount).to.be.equal(0);
    await expect(getUTXO({})).to.be.rejectedWith('should have required property \'address\'');
    expect(spy.callCount).to.be.equal(0);
    await expect(getUTXO({ address: 1 })).to.be.rejectedWith('params.address should be array,string');
    expect(spy.callCount).to.be.equal(0);
  });
});
