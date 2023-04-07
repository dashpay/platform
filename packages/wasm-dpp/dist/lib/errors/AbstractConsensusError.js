"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.AbstractConsensusError = void 0;
const DPPError_1 = require("./DPPError");
/**
 * @abstract
 */
class AbstractConsensusError extends DPPError_1.DPPError {
    /**
     * @param {string} message
     */
    constructor(message) {
        super(message);
    }
}
exports.AbstractConsensusError = AbstractConsensusError;
