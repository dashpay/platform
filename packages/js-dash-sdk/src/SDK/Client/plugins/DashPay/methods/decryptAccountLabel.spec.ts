import 'mocha';
import { expect } from 'chai';
import { decryptAccountLabel } from "./decryptAccountLabel";

describe('DashPayPlugin - decryptAccountLabel', () => {
  it('should decrypt an account label', function () {
    const sharedSecret = '0ec54a54b97988862cadf92b0f09337f9aabee0ecfbedaac23a635264a3a39e5';
    const accountLabel = 'Default account';
    const encryptedAccountLabel = '04UeDNhOFc8MjQNIACelmpIoqhEqB/A4trykL/ErXftqzuYS5KbduZhLH9wDiHoA';

    expect(decryptAccountLabel(encryptedAccountLabel, sharedSecret)).to.deep.equal(accountLabel);
  });

});
