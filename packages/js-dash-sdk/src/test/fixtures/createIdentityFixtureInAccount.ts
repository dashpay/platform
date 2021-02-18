import IdentityPublicKey from "@dashevo/dpp/lib/identity/IdentityPublicKey";
// @ts-ignore
const getIdentityFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityFixture');

export function createIdentityFixtureInAccount(account) {
    const identityFixture = getIdentityFixture();
    const identityFixtureIndex = 10000;
    const { privateKey: identityPrivateKey } = account.identities.getIdentityHDKeyByIndex(identityFixtureIndex, 0);

    identityFixture.publicKeys[0] = new IdentityPublicKey({
        id: 0,
        type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
        data: identityPrivateKey.toPublicKey().toBuffer(),
    });

    account.storage.insertIdentityIdAtIndex(
        account.walletId,
        identityFixture.getId().toString(),
        identityFixtureIndex,
    );

    return identityFixture;
}