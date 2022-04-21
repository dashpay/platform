import 'mocha';
import { expect } from 'chai';
import { decryptPublicKey } from "./decryptPublicKey";


describe('DashPayPlugin - decryptAccountLabel', () => {
  it('should decrypt an account label', function () {
    const encryptedPublicKey = 'd11e922c0d5259c614b17c764193714a23a84eeb3b25c60d3d5115c057fb6d47c7d26507445d81bed3c06bdfc592c280ebd021f386734a3228643707fa209b0c45b54b76680f4e9f047ee4e50fc987aa74c705b4039bb545511f5a87009e7b46';
    const expectedPublicKey = 'e2b33811c9b15725c7ca44cc331e3aa80e3ffbd2ec71ddf029e48b8e56b9cee165c9114d0327a6821cd96375604b69ce1ef96e32d4da479f56e5e9f647907ae01f541a4733';
    const sharedSecret = '0ec54a54b97988862cadf92b0f09337f9aabee0ecfbedaac23a635264a3a39e5';

    const encryptedPublicKeyBuffer = Buffer.from(encryptedPublicKey, 'hex');
    expect(decryptPublicKey(encryptedPublicKeyBuffer, sharedSecret)).to.deep.equal(expectedPublicKey);
  });

});
