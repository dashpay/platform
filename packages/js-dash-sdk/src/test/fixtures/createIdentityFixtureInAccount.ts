import loadWasmDpp from '@dashevo/wasm-dpp';
const { IdentityPublicKey } = loadWasmDpp;
import getIdentityFixture from '@dashevo/wasm-dpp/lib/test/fixtures/getIdentityFixture.js';

export async function createIdentityFixtureInAccount(account) {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const load = (loadWasmDpp as any).default ?? loadWasmDpp;
  await load();

  const identityFixture = await getIdentityFixture();
  const identityFixtureIndex = 0;
  const { privateKey: identityMasterPrivateKey } = account
    .identities.getIdentityHDKeyByIndex(identityFixtureIndex, 0);
  const { privateKey: identitySecondPrivateKey } = account
    .identities.getIdentityHDKeyByIndex(identityFixtureIndex, 1);
  const { privateKey: identityThirdPrivateKey } = account
    .identities.getIdentityHDKeyByIndex(identityFixtureIndex, 2);

  const publicKeyOne = new IdentityPublicKey(1);
  publicKeyOne.setData(identityMasterPrivateKey.toPublicKey().toBuffer());
  publicKeyOne.setSecurityLevel(IdentityPublicKey.SECURITY_LEVELS.MASTER);

  const publicKeyTwo = new IdentityPublicKey(1);
  publicKeyTwo.setId(1);
  publicKeyTwo.setData(identitySecondPrivateKey.toPublicKey().toBuffer());
  publicKeyTwo.setSecurityLevel(IdentityPublicKey.SECURITY_LEVELS.HIGH);

  const publicKeyThree = new IdentityPublicKey(1);
  publicKeyThree.setId(2);
  publicKeyThree.setData(identityThirdPrivateKey.toPublicKey().toBuffer());
  publicKeyThree.setSecurityLevel(IdentityPublicKey.SECURITY_LEVELS.CRITICAL);

  identityFixture.setPublicKeys([publicKeyOne, publicKeyTwo, publicKeyThree]);

  account.storage
    .getWalletStore(account.walletId)
    .insertIdentityIdAtIndex(
      identityFixture.getId().toString(),
      identityFixtureIndex,
    );

  return identityFixture;
}
