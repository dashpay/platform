const Address = require('@dashevo/dashcore-lib/lib/address');
const Script = require('@dashevo/dashcore-lib/lib/script');

const getPublicKeyFromPayoutScript = require('../../../../lib/identity/masternode/getPublicKeyFromPayoutScript');

describe('getPublicKeyFromPayoutScript', () => {
  let IdentityPublicKey;
  let InvalidIdentityPublicKeyTypeError;
  let KeyType;

  before(function before() {
    ({ IdentityPublicKey, InvalidIdentityPublicKeyTypeError, KeyType } = this.dppWasm);
  });

  it('should return public key for ECDSA_HASH160 script', function test() {
    const payoutAddress = Address.fromString('yLceJztHVZFbeqE9v86sLD9bDKFBmNqHQD');
    const scriptBuffer = new Script(payoutAddress);

    const type = KeyType.ECDSA_HASH160;

    const result = getPublicKeyFromPayoutScript(scriptBuffer, type, this.dppWasm);

    expect(result).to.deep.equal(Buffer.from('0340a3abf7e6eccf42b4dd71ef8c20ed53a78d1f', 'hex'));
  });

  it('should return public key for BIP13_SCRIPT_HASH script', function test() {
    const payoutAddress = Address.fromString('7UkJidhNjEPJCQnCTXeaJKbJmL4JuyV66w');
    const scriptBuffer = new Script(payoutAddress);

    const type = KeyType.BIP13_SCRIPT_HASH;

    const result = getPublicKeyFromPayoutScript(scriptBuffer, type, this.dppWasm);

    expect(result).to.deep.equal(Buffer.from('19a7d869032368fd1f1e26e5e73a4ad0e474960e', 'hex'));
  });

  it('should throw InvalidIdentityPublicKeyTypeError if type is unknown', function test() {
    const payoutAddress = Address.fromString('7UkJidhNjEPJCQnCTXeaJKbJmL4JuyV66w');
    const scriptBuffer = new Script(payoutAddress);

    try {
      getPublicKeyFromPayoutScript(scriptBuffer, -1, this.dppWasm);

      expect.fail('should throw InvalidIdentityPublicKeyTypeError');
    } catch (e) {
      expect(e).to.be.an.instanceof(InvalidIdentityPublicKeyTypeError);
      expect(e.getPublicKeyType()).to.equal(-1);
    }
  });
});
