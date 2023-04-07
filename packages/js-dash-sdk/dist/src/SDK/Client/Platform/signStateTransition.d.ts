import { Platform } from './Platform';
/**
 *
 * @param {Platform} platform
 * @param {AbstractStateTransition} stateTransition
 * @param {Identity} identity
 * @param {number} [keyIndex]
 * @return {AbstractStateTransition}
 */
export declare function signStateTransition(platform: Platform, stateTransition: any, identity: any, keyIndex?: number): Promise<any>;
