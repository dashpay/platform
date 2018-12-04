const chai = require('chai');
const chaiAsPromised = require('chai-as-promised');
const sinon = require('sinon');
const getQuorumFactory = require('../../../lib/rpcServer/commands/getQuorum');
const coreApiFixture = require('../../fixtures/coreAPIFixture');

chai.use(chaiAsPromised);
const { expect } = chai;
let spy;

describe('fetchDapContract', () => {
  describe('#factory', () => {
    it('should return a function', () => {
      const getDapContract = getQuorumFactory(coreApiFixture);
      expect(getDapContract).to.be.a('function');
    });
  });

  before(() => {
    spy = sinon.spy(coreApiFixture, 'getQuorum');
  });

  beforeEach(() => {
    spy.resetHistory();
  });

  after(() => {
    spy.restore();
  });

  it('Should return assigned quorum with proofs', async () => {
    const qFactory = getQuorumFactory(coreApiFixture);
    expect(spy.callCount).to.be.equal(0);
    const randomId = 'c4ba45dcdfe2461e17a54d43ce12751c16cefd61';
    const qResult = await qFactory({ regTxId: randomId });
    expect(qResult).to.be.an('object');
    expect(spy.callCount).to.be.equal(1);


    const quorumMember = qResult.quorum[0];
    const { proofs } = qResult;

    expect(quorumMember.isValid).to.be.equal(true);
    expect(quorumMember.keyIDOperator).to.be.equal('e6be850bfe045d2cd2b0e5789010b1a910dd7d27');
    expect(quorumMember.keyIDVoting).to.be.equal('e6be850bfe045d2cd2b0e5789010b1a910dd7d27');
    expect(quorumMember.proRegTxHash).to.be.equal('3450cdbaa92432dd19672738342cb4f2467f1a8b142c31142ea39e14f3ab8c18');
    expect(quorumMember.service).to.be.equal('165.227.144.38:19999');
    expect(proofs.merkleHashes).to.be.an('array');
    expect(proofs.merkleFlags).to.be.equal(0x1d);
  });

  it('Should throw an error if arguments are not valid', async () => {
    // TODO: The following wasn't used. Consider removing.
    // const regTxId = 'c4ba45dcdfe2461e17a54d43ce12751c16cefd61';
    const getDapContract = await getQuorumFactory(coreApiFixture);
    expect(spy.callCount).to.be.equal(0);
    await expect(getDapContract({ regTxId: 0 })).to.be.rejectedWith('should be string');
    expect(spy.callCount).to.be.equal(0);
  });
});
