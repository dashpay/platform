import IdentityPublicKey from "@dashevo/dpp/lib/identity/IdentityPublicKey";
// @ts-ignore
const getIdentityFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityFixture');

export function createIdentityFixtureInAccount(account) {
    const identityFixture = getIdentityFixture();
    const identityFixtureIndex = 0;
    const { privateKey: identityMasterPrivateKey } = account.identities.getIdentityHDKeyByIndex(identityFixtureIndex, 0);
    const { privateKey: identitySecondPrivateKey } = account.identities.getIdentityHDKeyByIndex(identityFixtureIndex, 1);

    identityFixture.publicKeys[0] = new IdentityPublicKey({
        id: 0,
        type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
        data: identityMasterPrivateKey.toPublicKey().toBuffer(),
        purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
        securityLevel: IdentityPublicKey.SECURITY_LEVELS.MASTER,
        readOnly: false,
    });

    identityFixture.publicKeys[1] = new IdentityPublicKey({
      id: 1,
      type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
      data: identitySecondPrivateKey.toPublicKey().toBuffer(),
      purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
      securityLevel: IdentityPublicKey.SECURITY_LEVELS.HIGH,
      readOnly: false,
    });

    account.storage
      .getWalletStore(account.walletId)
      .insertIdentityIdAtIndex(
        identityFixture.getId().toString(),
        identityFixtureIndex,
    );

    return identityFixture;
}
