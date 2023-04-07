import { Platform } from '../../Platform';
declare let Identifier: any;
/**
 * Get an identity from the platform
 *
 * @param {Platform} this - bound instance class
 * @param {string|Identifier} id - id
 * @returns Identity
 */
export declare function get(this: Platform, id: typeof Identifier | string): Promise<any>;
export default get;
