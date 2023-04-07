import { Platform } from '../../Platform';
/**
 * Publish contract onto the platform
 *
 * @param {Platform} this - bound instance class
 * @param {DataContract} dataContract - contract
 * @param identity - identity
 * @return {DataContractUpdateTransition}
 */
export default function update(this: Platform, dataContract: any, identity: any): Promise<any>;
