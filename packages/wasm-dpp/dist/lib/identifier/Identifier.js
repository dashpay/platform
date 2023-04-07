"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.Identifier = void 0;
const bs58_1 = __importDefault(require("bs58"));
const IdentifierError_1 = __importDefault(require("./errors/IdentifierError"));
/**
 * @param {Buffer} buffer
 * @returns {Identifier}
 * @constructor
 */
class Identifier {
    constructor(buffer) {
        if (!Buffer.isBuffer(buffer)) {
            throw new IdentifierError_1.default('Identifier expects Buffer');
        }
        if (buffer.length !== 32) {
            throw new IdentifierError_1.default('Identifier must be 32 long');
        }
        const patchedBuffer = Buffer.from(buffer);
        Object.setPrototypeOf(patchedBuffer, Identifier.prototype);
        // noinspection JSValidateTypes
        // @ts-ignore
        return patchedBuffer;
    }
    /**
     * Convert to Buffer
     *
     * @return {Buffer}
     */
    toBuffer() {
        // @ts-ignore
        return Buffer.from(this);
    }
    /**
     * Encode to CBOR
     *
     * @param {CborEncoder} encoder
     * @return {boolean}
     */
    encodeCBOR(encoder) {
        encoder.pushAny(this.toBuffer());
        return true;
    }
    /**
     * Convert to JSON
     *
     * @return {string}
     */
    toJSON() {
        return this.toString();
    }
    /**
     * Encode to string
     *
     * @param {string} [encoding=base58]
     * @return {string}
     */
    toString(encoding = 'base58') {
        if (encoding === 'base58') {
            return bs58_1.default.encode(this.toBuffer());
        }
        return this.toBuffer().toString(encoding);
    }
    /**
     * Create Identifier from buffer or encoded string
     *
     * @param {string|Buffer} value
     * @param {string} encoding
     * @return {Identifier}
     */
    static from(value, encoding = undefined) {
        let buffer;
        if (typeof value === 'string') {
            if (encoding === undefined) {
                // eslint-disable-next-line no-param-reassign
                encoding = 'base58';
            }
            if (encoding === 'base58') {
                buffer = bs58_1.default.decode(value);
            }
            else {
                buffer = Buffer.from(value, 'base64');
            }
        }
        else {
            if (encoding !== undefined) {
                throw new IdentifierError_1.default('encoding accepted only with type string');
            }
            buffer = value;
        }
        return new Identifier(buffer);
    }
}
exports.Identifier = Identifier;
Identifier.MEDIA_TYPE = 'application/x.dash.dpp.identifier';
Object.setPrototypeOf(Identifier.prototype, Buffer.prototype);
exports.default = Identifier;
