import 'mocha';
import { expect } from 'chai';
import { encryptPublicKey } from "./encryptPublicKey";

describe('DashPayPlugin - encryptPublicKey', () => {
  it('should encrypt a publicKey', function () {
    const sharedSecret = '0ec54a54b97988862cadf92b0f09337f9aabee0ecfbedaac23a635264a3a39e5';
    const extendedPublicKeyBuffers = Buffer.from('e2b33811c9b15725c7ca44cc331e3aa80e3ffbd2ec71ddf029e48b8e56b9cee165c9114d0327a6821cd96375604b69ce1ef96e32d4da479f56e5e9f647907ae01f541a4733', 'hex');
    const encryptedPublicKey = encryptPublicKey(extendedPublicKeyBuffers, sharedSecret);
    expect(encryptedPublicKey.length).to.equal(192);
  });

});
