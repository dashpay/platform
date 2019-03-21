const chai = require('chai');
const chaiAsPromised = require('chai-as-promised');
const sinon = require('sinon');
const sendRawTransactionFactory = require('../../../../lib/rpcServer/commands/sendRawTransaction');
const coreAPIFixture = require('../../../mocks/coreAPIFixture');

chai.use(chaiAsPromised);
const { expect } = chai;
let spy;

const validHexTransaction = '02000000000140420f000000000062b94c5f000000000103626f62c8682a35436e4f3f63c37cbea48f10648084b675411b0f494f3b4a2b6d2d26e166c7fb562fa2a168b5e0eb93357339124dcba3e40b125b9344fbe5a2ab3c6c42353d334a8c5fa917e0b730ef824aa37952680d7281b400000000';

describe('sendRawTransaction', () => {
  describe('#factory', () => {
    it('should return a function', () => {
      const sendRawTransaction = sendRawTransactionFactory(coreAPIFixture);
      expect(sendRawTransaction).to.be.a('function');
    });
  });

  before(() => {
    spy = sinon.spy(coreAPIFixture, 'sendRawTransaction');
  });

  beforeEach(() => {
    spy.resetHistory();
  });

  after(async () => {
    spy.restore();
  });

  it('Should return a string', async () => {
    const sendRawTransaction = sendRawTransactionFactory(coreAPIFixture);
    expect(spy.callCount).to.be.equal(0);
    const txid = await sendRawTransaction({ rawTransaction: validHexTransaction });
    expect(txid).to.be.a('string');
    expect(spy.callCount).to.be.equal(1);
  });

  it('Should throw if arguments are not valid', async () => {
    const sendRawTransaction = sendRawTransactionFactory(coreAPIFixture);
    expect(spy.callCount).to.be.equal(0);
    await expect(sendRawTransaction([])).to.be.rejected;
    expect(spy.callCount).to.be.equal(0);
    await expect(sendRawTransaction({})).to.be.rejectedWith('should have required property \'rawTransaction\'');
    expect(spy.callCount).to.be.equal(0);
    await expect(sendRawTransaction({ rawTransaction: 1 })).to.be.rejectedWith('rawTransaction should be string');
    expect(spy.callCount).to.be.equal(0);
    await expect(sendRawTransaction({ rawTransaction: 'thisisnotvalidhex' })).to.be.rejectedWith('rawTransaction should match pattern "^(0x|0X)?[a-fA-F0-9]+$"');
    expect(spy.callCount).to.be.equal(0);
  });
});
