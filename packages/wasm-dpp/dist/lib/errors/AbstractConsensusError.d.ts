import { DPPError } from './DPPError';
/**
 * @abstract
 */
export declare class AbstractConsensusError extends DPPError {
    /**
     * @param {string} message
     */
    constructor(message: string);
}
