import { PrivateKey } from '@dashevo/dashcore-lib';
import { Platform } from '../../../Platform';
/**
 * Creates a funding transaction for the platform identity
 *  and returns one-time key to sign the state transition
 * @param {Platform} this
 * @param {AssetLockProof} assetLockProof - asset lock transaction proof
 *  for the identity create transition
 * @param {PrivateKey} assetLockPrivateKey - private key used in asset lock
 * @return {{identity: Identity, identityCreateTransition: IdentityCreateTransition}}
 *  - identity, state transition and index of the key used to create it
 * that can be used to sign registration/top-up state transition
 */
export declare function createIdentityCreateTransition(this: Platform, assetLockProof: any, assetLockPrivateKey: PrivateKey): Promise<{
    identity: any;
    identityCreateTransition: any;
    identityIndex: number;
}>;
export default createIdentityCreateTransition;
