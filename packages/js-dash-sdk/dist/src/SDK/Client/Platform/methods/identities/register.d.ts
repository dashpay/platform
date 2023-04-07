import { Platform } from '../../Platform';
/**
 * Register identities to the platform
 *
 * @param {number} [fundingAmount=1000000] - funding amount in duffs
 * @returns {Identity} identity - a register and funded identity
 */
export default function register(this: Platform, fundingAmount?: number): Promise<any>;
