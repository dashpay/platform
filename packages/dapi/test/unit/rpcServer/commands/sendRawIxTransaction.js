const chai = require('chai');
const chaiAsPromised = require('chai-as-promised');
const sinon = require('sinon');
const sendRawIxTransactionFactory = require('../../../../lib/rpcServer/commands/sendRawIxTransaction');
const coreAPIFixture = require('../../../mocks/coreAPIFixture');

chai.use(chaiAsPromised);
const { expect } = chai;
let spy;

const validIxHexTransaction = '0100000001183b828bf36d76b0375e963279fb945b4391f47e5182d581fe8a9914d1ba7f8a000000006b483045022100d844a9fa6e3ce47104fb9a6c3c079d87498fd425c6a57145fca95757a5128dd60220609af63fbe789baa78c46b792f82fab1efd23c40bf157e0228063b0dc85dd9260121023326f6a019328c865f862fef554a3f424908cf2395d3bf91302f9174d74f49f0ffffffff0210270000000000001976a9142fd0e16c05bbbcdc388d4807b5cbe5f45389eb2d88ac60489800000000001976a9149effe59e047c7e19b26ce5a6275c3a3dbc7286f588ac00000000';

describe('sendRawIxTransaction', () => {
  describe('#factory', () => {
    it('should return a function', () => {
      const sendRawIxTransaction = sendRawIxTransactionFactory(coreAPIFixture);
      expect(sendRawIxTransaction).to.be.a('function');
    });
  });

  before(() => {
    spy = sinon.spy(coreAPIFixture, 'sendRawIxTransaction');
  });

  beforeEach(() => {
    spy.resetHistory();
  });

  after(async () => {
    spy.restore();
  });

  it('Should return a string', async () => {
    const sendRawIxTransaction = sendRawIxTransactionFactory(coreAPIFixture);
    expect(spy.callCount).to.be.equal(0);
    const txid = await sendRawIxTransaction({ rawTransaction: validIxHexTransaction });
    expect(txid).to.be.a('string');
    expect(spy.callCount).to.be.equal(1);
  });

  it('Should throw if arguments are not valid', async () => {
    const sendRawIxTransaction = sendRawIxTransactionFactory(coreAPIFixture);
    expect(spy.callCount).to.be.equal(0);
    await expect(sendRawIxTransaction([])).to.be.rejected;
    expect(spy.callCount).to.be.equal(0);
    await expect(sendRawIxTransaction({})).to.be.rejectedWith('should have required property \'rawTransaction\'');
    expect(spy.callCount).to.be.equal(0);
    await expect(sendRawIxTransaction({ rawTransaction: 1 })).to.be.rejectedWith('rawTransaction should be string');
    expect(spy.callCount).to.be.equal(0);
    await expect(sendRawIxTransaction({ rawTransaction: 'thisisnotvalidhex' })).to.be.rejectedWith('rawTransaction should match pattern "^(0x|0X)?[a-fA-F0-9]+$"');
    expect(spy.callCount).to.be.equal(0);
  });
});
