import { Platform } from './Platform';
import { IStateTransitionResult } from './IStateTransitionResult';
/**
 * @param {Platform} platform
 * @param {Object} [options]
 * @param {boolean} [options.skipValidation=false]
 *
 * @param stateTransition
 */
export default function broadcastStateTransition(platform: Platform, stateTransition: any, options?: {
    skipValidation?: boolean;
}): Promise<IStateTransitionResult | void>;
