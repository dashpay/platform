const { expect } = require('chai');
const cbor = require('cbor');
const decrypt = require('./decrypt');

const jsonObject = {
  string: 'string',
  list: ['a', 'b', 'c', 'd'],
  obj: {
    int: 1,
    boolean: true,
    theNull: null,
  },
};
const secret = 'secret';

const extPubKey = 'tpubDFFUrLDh4ihrmngy3Mb1fv6LS76tZPeYEvXBrqrMGoog7o1VAdYj8Nu8J8VZaMcRE3ypL59N51namngy8ek1kun2ZFPLKEmS5uBBfyvGcpN';
const encryptedExtPubKey = 'U2FsdGVkX19YzY39phhQiw/mqBwAQKEYmXzw9PN6tQ8LtpQCIiGCfhYTUJEoXKQuaCug+ACuRe0C6iiFbe/7BfKNpvULK6lFFdaKjrfvNfWCCZKvBDXMVBX4u0uLWNcgcWEke/rMMAKex6Gt5UkdZd4BTv3pEiOay3YCDbtu9bY=';
const encryptedJSON = 'U2FsdGVkX19jvOuQ0Y7yJGiKDx/t1zoz3IDdlIS7uMyN7V5IUFvHMuD8D3QfoUcPOKBqTZcd8J2q3DRMC0h/5xa86ntm8gypxRCGd1IEAOFSZe9fWoW3qOW+JNOOekJGcEErFz28mffp/g0rThB14NwWDUivBNboCOZgABKJ0bS6OA/Lbcokl4+iDDCoRhkC';

describe('Account - decrypt', function suite() {
  this.timeout(10000);
  it('should decrypt extPubKey with aes', () => {
    const decryptedExtPubKey = decrypt('aes', encryptedExtPubKey, secret);
    expect(decryptedExtPubKey).to.equal(extPubKey);
  });
  it('should decrypt encoded json', () => {
    const decryptedJSON = decrypt('aes', encryptedJSON, secret);
    const decodedJSON = cbor.decodeFirstSync(decryptedJSON);
    expect(decodedJSON).to.deep.equal(jsonObject);
  });
});
