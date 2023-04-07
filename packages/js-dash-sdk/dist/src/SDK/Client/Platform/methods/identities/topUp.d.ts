import Identifier from '@dashevo/dpp/lib/Identifier';
import { Platform } from '../../Platform';
/**
 * Register identities to the platform
 *
 * @param {Platform} this - bound instance class
 * @param {Identifier|string} identityId - id of the identity to top up
 * @param {number} amount - amount to top up in duffs
 * @returns {boolean}
 */
export declare function topUp(this: Platform, identityId: Identifier | string, amount: number): Promise<any>;
export default topUp;
