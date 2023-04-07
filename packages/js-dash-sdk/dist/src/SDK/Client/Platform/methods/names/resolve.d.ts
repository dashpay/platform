import { Platform } from '../../Platform';
/**
 * This method will allow you to resolve a DPNS record from its humanized name.
 * @param {string} name - the exact alphanumeric (2-63) value used for human-identification
 * @returns {Document} document
 */
export declare function resolve(this: Platform, name: string): Promise<any>;
export default resolve;
