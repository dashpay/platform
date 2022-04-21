import 'mocha';
import { expect } from 'chai';
import { encryptAccountLabel } from "./encryptAccountLabel";

describe('DashPayPlugin - encryptAccountLabel', () => {
  it('should encrypt an account label', function () {
    const sharedSecret = '0ec54a54b97988862cadf92b0f09337f9aabee0ecfbedaac23a635264a3a39e5';
    const accountLabel = 'Default account';
    const cipherIv = 'd3851e0cd84e15cf0c8d03480027a59a';

    expect(encryptAccountLabel(sharedSecret, accountLabel, cipherIv)).to.deep.equal('04UeDNhOFc8MjQNIACelmpIoqhEqB/A4trykL/ErXftqzuYS5KbduZhLH9wDiHoA');
  });

});
