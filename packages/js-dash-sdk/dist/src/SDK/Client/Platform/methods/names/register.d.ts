import { Platform } from '../../Platform';
declare let Identifier: any;
/**
 * Register names to the platform
 *
 * @param {Platform} this - bound instance class
 * @param {string} name - name
 * @param {Object} records - records object having only one of the following items
 * @param {string} [records.dashUniqueIdentityId]
 * @param {string} [records.dashAliasIdentityId]
 * @param identity - identity
 *
 * @returns registered domain document
 */
export declare function register(this: Platform, name: string, records: {
    dashUniqueIdentityId?: typeof Identifier | string;
    dashAliasIdentityId?: typeof Identifier | string;
}, identity: {
    getId(): typeof Identifier;
    getPublicKeyById(number: number): any;
}): Promise<any>;
export default register;
