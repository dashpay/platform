const { expect } = require('chai');
const getTotalBalance = require('./getTotalBalance');
const getConfirmedBalance = require('./getConfirmedBalance');
const getUnconfirmedBalance = require('./getUnconfirmedBalance');
const getFixtureHDAccountWithStorage = require("../../../../fixtures/wallets/apart-trip-dignity/getFixtureAccountWithStorage");

let mockedAccount;
describe('Account - getUnconfirmedBalance', function suite() {
  this.timeout(10000);
  before(() => {
    mockedAccount = getFixtureHDAccountWithStorage();
  });

  it('should correctly get the balance', () => {
    const balance = getTotalBalance.call(mockedAccount);
    expect(balance).to.equal(667198249);
  });

  it('should correctly get the balance confirmed only', () => {
    const balance = getConfirmedBalance.call(mockedAccount);
    expect(balance).to.equal(667198249);
  });

  // TODO: file looks like a complete duplicate of the getTotalBalance.spec.js
  // Should we actually mock and test unconfirmed balance?
  it('should correctly get the balance dash value instead of duff', () => {
    const balanceTotalDash = getTotalBalance.call(mockedAccount, false);
    const balanceUnconfDash = getUnconfirmedBalance.call(mockedAccount, false);
    const balanceConfDash = getConfirmedBalance.call(mockedAccount, false);

    expect(balanceTotalDash).to.equal(6.67198249);
    expect(balanceUnconfDash).to.equal(0);
    expect(balanceConfDash).to.equal(6.67198249);
  });
});
