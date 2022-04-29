const IdentityPublicKey = require('@dashevo/dpp/lib/identity/IdentityPublicKey');
const Address = require('@dashevo/dashcore-lib/lib/address');
const Script = require('@dashevo/dashcore-lib/lib/script');
const getWithdrawPubKeyTypeFromPayoutScriptFactory = require('../../../../lib/identity/masternode/getWithdrawPubKeyTypeFromPayoutScriptFactory');
const InvalidPayoutScriptError = require('../../../../lib/identity/masternode/errors/InvalidPayoutScriptError');

describe('getWithdrawPubKeyTypeFromPayoutScriptFactory', () => {
  let getWithdrawPubKeyTypeFromPayoutScript;
  let network;

  beforeEach(() => {
    network = 'testnet';
    getWithdrawPubKeyTypeFromPayoutScript = getWithdrawPubKeyTypeFromPayoutScriptFactory(
      network,
    );
  });

  it('should return ECDSA_HASH160 if address has p2pkh type', () => {
    const payoutScript = Script(Address.fromString('yTsGq4wV8WF5GKLaYV2C43zrkr2sfTtysT')).toBuffer();
    const type = getWithdrawPubKeyTypeFromPayoutScript(payoutScript);

    expect(type).to.be.equal(IdentityPublicKey.TYPES.ECDSA_HASH160);
  });

  it('should return BIP13_SCRIPT_HASH if address has p2sh type', () => {
    const payoutScript = Script(Address.fromString('7UkJidhNjEPJCQnCTXeaJKbJmL4JuyV66w')).toBuffer();
    const type = getWithdrawPubKeyTypeFromPayoutScript(payoutScript);

    expect(type).to.be.equal(IdentityPublicKey.TYPES.BIP13_SCRIPT_HASH);
  });

  it('should throw InvalidPayoutScriptError if address is not p2sh or p2pkh', () => {
    const payoutScript = Buffer.alloc(23);

    try {
      getWithdrawPubKeyTypeFromPayoutScript(payoutScript);

      expect.fail('should throw InvalidPayoutScriptError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InvalidPayoutScriptError);
      expect(e.getPayoutScript()).to.deep.equal(payoutScript);
    }
  });
});
