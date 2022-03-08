const { expect } = require('chai');
const getTotalBalance = require('./getTotalBalance');
const getConfirmedBalance = require('./getConfirmedBalance');
const getUnconfirmedBalance = require('./getUnconfirmedBalance');
const mockAccountWithStorage = require("../../../test/mocks/mockAccountWithStorage");

let mockedAccount;
describe('Account - getUnconfirmedBalance', function suite() {
  this.timeout(10000);
  before(() => {
    mockedAccount = mockAccountWithStorage()
  });

  it('should correctly get the balance', () => {
    const balance = getTotalBalance.call(mockedAccount);
    expect(balance).to.equal(224108673);
  });

  it('should correctly get the balance confirmed only', () => {
    const balance = getConfirmedBalance.call(mockedAccount);
    expect(balance).to.equal(224108673);
  });

  // TODO: file looks like a complete duplicate of the getTotalBalance.spec.js
  // Should we actually mock and test unconfirmed balance?
  it('should correctly get the balance dash value instead of duff', () => {
    const balanceTotalDash = getTotalBalance.call(mockedAccount, false);
    const balanceUnconfDash = getUnconfirmedBalance.call(mockedAccount, false);
    const balanceConfDash = getConfirmedBalance.call(mockedAccount, false);

    expect(balanceTotalDash).to.equal(2.24108673);
    expect(balanceUnconfDash).to.equal(0);
    expect(balanceConfDash).to.equal(2.24108673);
  });
});
