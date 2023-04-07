import Identity from '@dashevo/dpp/lib/identity/Identity';
import IdentityPublicKey from '@dashevo/dpp/lib/identity/IdentityPublicKey';
import { Platform } from '../../Platform';
/**
 * Update platform identities
 *
 * @param {Platform} this - bound instance class
 * @param {Identity} identity - identity to update
 * @param {{add: IdentityPublicKey[]; disable: IdentityPublicKey[]}} publicKeys - public keys to add
 * @param {Object<string, any>} privateKeys - public keys to add
 *
 * @returns {boolean}
 */
export declare function update(this: Platform, identity: Identity, publicKeys: {
    add?: IdentityPublicKey[];
    disable?: IdentityPublicKey[];
}, privateKeys: {
    string: any;
    any: any;
}): Promise<any>;
export default update;
