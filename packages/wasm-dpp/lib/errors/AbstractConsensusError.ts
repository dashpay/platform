import { DPPError } from './DPPError'

/**
 * @abstract
 */
export class AbstractConsensusError extends DPPError {
    /**
     * @param {string} message
     */
    constructor(message: string) {
        super(message);
    }
}
