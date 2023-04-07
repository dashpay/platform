import { Platform } from '../../Platform';
/**
 * Publish contract onto the platform
 *
 * @param {Platform} this - bound instance class
 * @param dataContract - contract
 * @param identity - identity
 * @return {DataContractCreateTransition}
 */
export default function publish(this: Platform, dataContract: any, identity: any): Promise<any>;
