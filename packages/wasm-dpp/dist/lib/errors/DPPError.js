"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.DPPError = void 0;
class DPPError extends Error {
    constructor(message) {
        super();
        this.name = this.constructor.name;
        this.message = message;
        if (Error.captureStackTrace) {
            Error.captureStackTrace(this, this.constructor);
        }
    }
}
exports.DPPError = DPPError;
