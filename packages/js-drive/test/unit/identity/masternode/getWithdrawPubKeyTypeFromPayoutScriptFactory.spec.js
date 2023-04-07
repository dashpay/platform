const Address = require('@dashevo/dashcore-lib/lib/address');
const Script = require('@dashevo/dashcore-lib/lib/script');
const getWithdrawPubKeyTypeFromPayoutScriptFactory = require('../../../../lib/identity/masternode/getWithdrawPubKeyTypeFromPayoutScriptFactory');
const InvalidPayoutScriptError = require('../../../../lib/identity/masternode/errors/InvalidPayoutScriptError');

describe('getWithdrawPubKeyTypeFromPayoutScriptFactory', () => {
  let getWithdrawPubKeyTypeFromPayoutScript;
  let network;
  let KeyType;

  before(function before() {
    ({ KeyType } = this.dppWasm);
  });

  beforeEach(function beforeEach() {
    network = 'testnet';
    getWithdrawPubKeyTypeFromPayoutScript = getWithdrawPubKeyTypeFromPayoutScriptFactory(
      network,
      this.dppWasm,
    );
  });

  it('should return ECDSA_HASH160 if address has p2pkh type', function test() {
    const payoutScript = Script(Address.fromString('yTsGq4wV8WF5GKLaYV2C43zrkr2sfTtysT'));
    const type = getWithdrawPubKeyTypeFromPayoutScript(payoutScript, this.dppWasm);

    expect(type).to.be.equal(KeyType.ECDSA_HASH160);
  });

  it('should return BIP13_SCRIPT_HASH if address has p2sh type', function test() {
    const payoutScript = Script(Address.fromString('7UkJidhNjEPJCQnCTXeaJKbJmL4JuyV66w'));
    const type = getWithdrawPubKeyTypeFromPayoutScript(payoutScript, this.dppWasm);

    expect(type).to.be.equal(KeyType.BIP13_SCRIPT_HASH);
  });

  it('should throw InvalidPayoutScriptError if address is not p2sh or p2pkh', function test() {
    const payoutScript = new Script();

    try {
      getWithdrawPubKeyTypeFromPayoutScript(payoutScript, this.dppWasm);

      expect.fail('should throw InvalidPayoutScriptError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InvalidPayoutScriptError);
      expect(e.getPayoutScript()).to.deep.equal(payoutScript);
    }
  });
});
