const chai = require('chai');
const chaiAsPromised = require('chai-as-promised');
const sinon = require('sinon');
const getUserFactory = require('../../../../lib/rpcServer/commands/getUser');
const coreAPIFixture = require('../../../mocks/coreAPIFixture');

chai.use(chaiAsPromised);
const { expect } = chai;
let spy;

describe('getUser', () => {
  describe('#factory', () => {
    it('should return a function', () => {
      const getUser = getUserFactory(coreAPIFixture);
      expect(getUser).to.be.a('function');
    });
  });

  before(() => {
    spy = sinon.spy(coreAPIFixture, 'getUser');
  });

  beforeEach(() => {
    spy.resetHistory();
  });

  after(() => {
    spy.restore();
  });

  it('Should return blockchain user', async () => {
    const getUser = getUserFactory(coreAPIFixture);
    expect(spy.callCount).to.be.equal(0);
    let user = await getUser({ username: 'alice' });
    expect(user).to.be.an('object');
    expect(spy.callCount).to.be.equal(1);
    user = await getUser({ userId: 'beef56cc3cff03a48d078fd7839c05ec16f12f1919ac366596bb5e025f78a2aa' });
    expect(user).to.be.an('object');
    expect(spy.callCount).to.be.equal(2);
  });

  it('Should throw an error if arguments are not valid', async () => {
    const getUser = getUserFactory(coreAPIFixture);
    expect(spy.callCount).to.be.equal(0);
    await expect(getUser({ username: 123 })).to.be.rejectedWith('should be string');
    expect(spy.callCount).to.be.equal(0);
    await expect(getUser({ userId: 123 })).to.be.rejectedWith('should be string');
    expect(spy.callCount).to.be.equal(0);
    await expect(getUser({})).to.be.rejectedWith('should have required property');
    expect(spy.callCount).to.be.equal(0);
    await expect(getUser()).to.be.rejectedWith('should be object');
    expect(spy.callCount).to.be.equal(0);
    // todo
    // await expect(getUser({
    // username: 'beef56cc3cff03a48d078fd7839c05ec16f12f1919ac366596bb5e025f78a2aa'
    // })).to.be.rejectedWith('should be integer');
    // expect(spy.callCount).to.be.equal(0);
    // todo
    // await expect(getUser({ userId: 'alice' })).to.be.rejectedWith('should be integer');
    // expect(spy.callCount).to.be.equal(0);
    await expect(getUser([-1])).to.be.rejected;
    expect(spy.callCount).to.be.equal(0);
  });
});
