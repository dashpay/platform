"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const DPPError_1 = require("../../errors/DPPError");
class IdentifierError extends DPPError_1.DPPError {
    constructor(message) {
        super(message);
    }
}
exports.default = IdentifierError;
