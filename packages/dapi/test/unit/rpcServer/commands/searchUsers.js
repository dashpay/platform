const chai = require('chai');
const chaiAsPromised = require('chai-as-promised');
const sinon = require('sinon');
const searchUsersFactory = require('../../../../lib/rpcServer/commands/searchUsers');
const userIndex = require('../../../mocks/userIndexFixture');

chai.use(chaiAsPromised);
const { expect } = chai;
let spy;

describe('searchUsers', () => {
  describe('#factory', () => {
    it('should return a function', () => {
      const searchUser = searchUsersFactory(userIndex);
      expect(searchUser).to.be.a('function');
    });
  });

  before(() => {
    spy = sinon.spy(userIndex, 'searchUsernames');
  });

  beforeEach(() => {
    spy.resetHistory();
  });

  after(() => {
    spy.restore();
  });

  it('Should return search results', async () => {
    const searchUsers = searchUsersFactory(userIndex);
    expect(spy.callCount).to.be.equal(0);
    const users = await searchUsers({ pattern: 'Dash', limit: 10, offset: 0 });
    expect(users).to.be.an('object');
    expect(users).to.have.property('totalCount');
    expect(users).to.have.property('results');
    expect(spy.callCount).to.be.equal(1);
  });

  it('Should throw an error if arguments are not valid', async () => {
    const searchUsers = searchUsersFactory(userIndex);
    expect(spy.callCount).to.be.equal(0);
    await expect(searchUsers({ pattern: 123, offset: 10, limit: 10 })).to.be.rejectedWith('should be string');
    expect(spy.callCount).to.be.equal(0);
    await expect(searchUsers({ pattern: 'Dash', offset: -1, limit: 10 })).to.be.rejectedWith('offset should be >= 0');
    expect(spy.callCount).to.be.equal(0);
    await expect(searchUsers({ pattern: 'Dash', offset: 10, limit: -1 })).to.be.rejectedWith('limit should be >= 1');
    expect(spy.callCount).to.be.equal(0);
    await expect(searchUsers({ pattern: 'Dash', offset: 10, limit: 0.5 })).to.be.rejectedWith('should be integer');
    expect(spy.callCount).to.be.equal(0);
    await expect(searchUsers({ pattern: 'Dash', offset: 0.5, limit: 10 })).to.be.rejectedWith('should be integer');
    expect(spy.callCount).to.be.equal(0);
    await expect(searchUsers({ pattern: 'Dash', offset: '10', limit: 10 })).to.be.rejectedWith('should be integer');
    expect(spy.callCount).to.be.equal(0);
    await expect(searchUsers({ pattern: 'Dash', offset: 20, limit: '10' })).to.be.rejectedWith('should be integer');
    expect(spy.callCount).to.be.equal(0);
    await expect(searchUsers({
      pattern: 'Dash',
      limit: 10,
    })).to.be.rejectedWith('should have required property \'offset\'');
    expect(spy.callCount).to.be.equal(0);
    await expect(searchUsers({
      pattern: 'Dash',
      offset: 10,
    })).to.be.rejectedWith('should have required property \'limit\'');
    expect(spy.callCount).to.be.equal(0);
    await expect(searchUsers({})).to.be.rejectedWith('should have required property');
    expect(spy.callCount).to.be.equal(0);
    await expect(searchUsers()).to.be.rejectedWith('should be object');
    expect(spy.callCount).to.be.equal(0);
    await expect(searchUsers([123])).to.be.rejected;
    expect(spy.callCount).to.be.equal(0);
    await expect(searchUsers([-1])).to.be.rejected;
    expect(spy.callCount).to.be.equal(0);
  });
});
