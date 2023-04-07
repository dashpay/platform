import { Platform } from '../../Platform';
declare let Identifier: any;
declare type ContractIdentifier = string | typeof Identifier;
/**
 * Get contracts from the platform
 *
 * @param {Platform} this - bound instance class
 * @param {ContractIdentifier} identifier - identifier of the contract to fetch
 * @returns contracts
 */
export declare function get(this: Platform, identifier: ContractIdentifier): Promise<any>;
export default get;
