/// <reference types="node" />
declare type CborEncoder = {
    pushAny: (buffer: Buffer) => void;
};
declare type IdentifierEncoding = BufferEncoding | 'base58';
/**
 * @param {Buffer} buffer
 * @returns {Identifier}
 * @constructor
 */
export declare class Identifier {
    static MEDIA_TYPE: string;
    constructor(buffer: Buffer);
    /**
     * Convert to Buffer
     *
     * @return {Buffer}
     */
    toBuffer(): Buffer;
    /**
     * Encode to CBOR
     *
     * @param {CborEncoder} encoder
     * @return {boolean}
     */
    encodeCBOR(encoder: CborEncoder): boolean;
    /**
     * Convert to JSON
     *
     * @return {string}
     */
    toJSON(): string;
    /**
     * Encode to string
     *
     * @param {string} [encoding=base58]
     * @return {string}
     */
    toString(encoding?: IdentifierEncoding): string;
    /**
     * Create Identifier from buffer or encoded string
     *
     * @param {string|Buffer} value
     * @param {string} encoding
     * @return {Identifier}
     */
    static from(value: string | Buffer, encoding?: string): Identifier;
}
export default Identifier;
