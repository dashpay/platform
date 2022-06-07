import { PrivateKey } from "@dashevo/dashcore-lib";
import { Platform } from "../../../Platform";
import IdentityPublicKey from "@dashevo/dpp/lib/identity/IdentityPublicKey"

/**
 * Creates a funding transaction for the platform identity and returns one-time key to sign the state transition
 * @param {Platform} this
 * @param {AssetLockProof} assetLockProof - asset lock transaction proof for the identity create transition
 * @param {PrivateKey} assetLockPrivateKey - private key used in asset lock
 * @param {string|Buffer|Identifier} identityId
 * @return {{identity: Identity, identityCreateTransition: IdentityCreateTransition}} - identity, state transition and index of the key used to create it
 * that can be used to sign registration/top-up state transition
 */
export async function createIdentityTopUpTransition(this : Platform, assetLockProof: any, assetLockPrivateKey: PrivateKey, identityId: any): Promise<any> {
    const platform = this;
    await platform.initialize();

    const { dpp } = platform;

    // @ts-ignore
    const identityTopUpTransition = dpp.identity.createIdentityTopUpTransition(
        identityId,  assetLockProof
    );

    await identityTopUpTransition.signByPrivateKey(assetLockPrivateKey, IdentityPublicKey.TYPES.ECDSA_SECP256K1);

    const result = await dpp.stateTransition.validateBasic(identityTopUpTransition);

    if (!result.isValid()) {
        throw new Error(`StateTransition is invalid - ${JSON.stringify(result.getErrors())}`);
    }

    return identityTopUpTransition;
}

export default createIdentityTopUpTransition;
