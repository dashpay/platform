import { expect } from 'chai';
import getTotalBalance from './getTotalBalance.js';
import getConfirmedBalance from './getConfirmedBalance.js';
import getUnconfirmedBalance from './getUnconfirmedBalance.js';
import getFixtureHDAccountWithStorage from '../../../../fixtures/wallets/apart-trip-dignity/getFixtureAccountWithStorage.js';

let mockedAccount;
describe('Account - getTotalBalance', function suite() {
  this.timeout(10000);
  before(() => {
    mockedAccount = getFixtureHDAccountWithStorage();
  });
  it('should correctly get the balance',() => {
    const balance = getTotalBalance.call(mockedAccount);
    expect(balance).to.equal(667198249);
  });
  it('should correctly get the balance confirmed only',  () => {
    const balance = getConfirmedBalance.call(mockedAccount);
    expect(balance).to.equal(667198249);
  });
  it('should correctly get the balance dash value instead of duff',  () => {
    const balanceTotalDash = getTotalBalance.call(mockedAccount, false);
    const balanceUnconfDash = getUnconfirmedBalance.call(mockedAccount, false);
    const balanceConfDash = getConfirmedBalance.call(mockedAccount, false);

    expect(balanceTotalDash).to.equal(6.67198249);
    expect(balanceUnconfDash).to.equal(0);
    expect(balanceConfDash).to.equal(6.67198249);
  });
});
