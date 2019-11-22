declare module "@dashevo/dashcore-lib" {
    /**
     * Instantiate an address from an address String or Buffer, a public key or script hash Buffer,
     * or an instance of {@link PublicKey} or {@link Script}.
     *
     * This is an immutable class, and if the first parameter provided to this constructor is an
     * `Address` instance, the same argument will be returned.
     *
     * An address has two key properties: `network` and `type`. The type is either
     * `Address.PayToPublicKeyHash` (value is the `'pubkeyhash'` string)
     * or `Address.PayToScriptHash` (the string `'scripthash'`). The network is an instance of {@link Network}.
     * You can quickly check whether an address is of a given kind by using the methods
     * `isPayToPublicKeyHash` and `isPayToScriptHash`
     *
     * @example
     * ```javascript
     * // validate that an input field is valid
     * var error = Address.getValidationError(input, 'testnet');
     * if (!error) {
     *   var address = Address(input, 'testnet');
     * } else {
     *   // invalid network or checksum (typo?)
     *   var message = error.messsage;
     * }
     *
     * // get an address from a public key
     * var address = Address(publicKey, 'testnet').toString();
     * ```
     *
     * @param {*} data - The encoded data in various formats
     * @param {Network|String|number=} network - The network: 'livenet' or 'testnet'
     * @param {string=} type - The type of address: 'script' or 'pubkey'
     * @returns {Address} A new valid and frozen instance of an Address
     * @constructor
     */
    export class Address {
        constructor(data: any, network: Network | string | number, type?: string);

        /**
         * Internal function used to split different kinds of arguments of the constructor
         * @param {*} data - The encoded data in various formats
         * @param {Network|String|number=} network - The network: 'livenet' or 'testnet'
         * @param {string=} type - The type of address: 'script' or 'pubkey'
         * @returns {Object} An "info" object with "type", "network", and "hashBuffer"
         */
        _classifyArguments(data: any, network: Network | string | number, type?: string): any;

        /**
         * @static
         * @type string
         */
        static PayToPublicKeyHash: string;
        /**
         * @static
         * @type string
         */
        static PayToScriptHash: string;

        /**
         * Deserializes an address serialized through `Address#toObject()`
         * @param {Object} data
         * @param {string} data.hash - the hash that this address encodes
         * @param {string} data.type - either 'pubkeyhash' or 'scripthash'
         * @param {Network=} data.network - the name of the network associated
         * @return {Address}
         */
        static _transformObject(data: {
            hash: string;
            type: string;
            network?: Network;
        }): Address;

        /**
         * Creates a P2SH address from a set of public keys and a threshold.
         *
         * The addresses will be sorted lexicographically, as that is the trend in bitcoin.
         * To create an address from unsorted public keys, use the {@link Script#buildMultisigOut}
         * interface.
         *
         * @param {Array} publicKeys - a set of public keys to create an address
         * @param {number} threshold - the number of signatures needed to release the funds
         * @param {String|Network} network - either a Network instance, 'livenet', or 'testnet'
         * @return {Address}
         */
        static createMultisig(publicKeys: any[], threshold: number, network: string | Network): Address;

        /**
         * Instantiate an address from a PublicKey instance
         *
         * @param {PublicKey} data
         * @param {String|Network} network - either a Network instance, 'livenet', or 'testnet'
         * @returns {Address} A new valid and frozen instance of an Address
         */
        static fromPublicKey(data: PublicKey, network: string | Network): Address;

        /**
         * Instantiate an address from a ripemd160 public key hash
         *
         * @param {Buffer} hash - An instance of buffer of the hash
         * @param {String|Network} network - either a Network instance, 'livenet', or 'testnet'
         * @returns {Address} A new valid and frozen instance of an Address
         */
        static fromPublicKeyHash(hash: Buffer, network: string | Network): Address;

        /**
         * Instantiate an address from a ripemd160 script hash
         *
         * @param {Buffer} hash - An instance of buffer of the hash
         * @param {String|Network} network - either a Network instance, 'livenet', or 'testnet'
         * @returns {Address} A new valid and frozen instance of an Address
         */
        static fromScriptHash(hash: Buffer, network: string | Network): Address;

        /**
         * Builds a p2sh address paying to script. This will hash the script and
         * use that to create the address.
         * If you want to extract an address associated with a script instead,
         * see {{Address#fromScript}}
         *
         * @param {Script} script - An instance of Script
         * @param {String|Network} network - either a Network instance, 'livenet', or 'testnet'
         * @returns {Address} A new valid and frozen instance of an Address
         */
        static payingTo(script: Script, network: string | Network): Address;

        /**
         * Extract address from a Script. The script must be of one
         * of the following types: p2pkh input, p2pkh output, p2sh input
         * or p2sh output.
         * This will analyze the script and extract address information from it.
         * If you want to transform any script to a p2sh Address paying
         * to that script's hash instead, use {{Address#payingTo}}
         *
         * @param {Script} script - An instance of Script
         * @param {String|Network} network - either a Network instance, 'livenet', or 'testnet'
         * @returns {Address} A new valid and frozen instance of an Address
         */
        static fromScript(script: Script, network: string | Network): Address;

        /**
         * Instantiate an address from a buffer of the address
         *
         * @param {Buffer} buffer - An instance of buffer of the address
         * @param {String|Network=} network - either a Network instance, 'livenet', or 'testnet'
         * @param {string=} type - The type of address: 'script' or 'pubkey'
         * @returns {Address} A new valid and frozen instance of an Address
         */
        static fromBuffer(buffer: Buffer, network: string | Network, type?: string): Address;

        /**
         * Instantiate an address from an address string
         *
         * @param {string} str - A string of the Bitcoin address
         * @param {String|Network=} network - either a Network instance, 'livenet', or 'testnet'
         * @param {string=} type - The type of address: 'script' or 'pubkey'
         * @returns {Address} A new valid and frozen instance of an Address
         */
        static fromString(str: string, network: string | Network, type?: string): Address;

        /**
         * Instantiate an address from an Object
         *
         * @param {string} json - An JSON string or Object with keys: hash, network and type
         * @returns {Address} A new valid instance of an Address
         */
        static fromObject(json: string): Address;

        /**
         * Will return a validation error if exists
         *
         * @example
         * ```javascript
         * // a network mismatch error
         * var error = Address.getValidationError('15vkcKf7gB23wLAnZLmbVuMiiVDc1Nm4a2', 'testnet');
         * ```
         *
         * @param {string} data - The encoded data
         * @param {String|Network} network - either a Network instance, 'livenet', or 'testnet'
         * @param {string} type - The type of address: 'script' or 'pubkey'
         * @returns {null|Error} The corresponding error message
         */
        static getValidationError(data: string, network: string | Network, type: string): null | Error;

        /**
         * Will return a boolean if an address is valid
         *
         * @example
         * ```javascript
         * assert(Address.isValid('15vkcKf7gB23wLAnZLmbVuMiiVDc1Nm4a2', 'livenet'));
         * ```
         *
         * @param {string} data - The encoded data
         * @param {String|Network} network - either a Network instance, 'livenet', or 'testnet'
         * @param {string} type - The type of address: 'script' or 'pubkey'
         * @returns {boolean} The corresponding error message
         */
        static isValid(data: string, network: string | Network, type: string): boolean;

        /**
         * Returns true if an address is of pay to public key hash type
         * @return {boolean}
         */
        isPayToPublicKeyHash(): boolean;

        /**
         * Returns true if an address is of pay to script hash type
         * @return {boolean}
         */
        isPayToScriptHash(): boolean;

        /**
         * Will return a buffer representation of the address
         *
         * @returns {Buffer} Bitcoin address buffer
         */
        toBuffer(): Buffer;

        /**
         * @function
         * @returns {Object} A plain object with the address information
         */
        toObject(): any;

        /**
         * Will return a string representation of the address
         *
         * @returns {string} Bitcoin address
         */
        toString(): string;

        /**
         * Will return a string formatted for the console
         *
         * @returns {string} Bitcoin address
         */
        inspect(): string;
    }

    /**
     * @param {Buffer|string|PartialMerkleTree|{transactionHashes: Buffer[],filterMatches: boolean[]}} [serialized]
     * @return {PartialMerkleTree}
     * @class
     * @property {number} totalTransactions
     * @property {string[]} merkleHashes
     * @property {number[]} merkleFlags
     */
    export class PartialMerkleTree {
        constructor(serialized?: Buffer | string | PartialMerkleTree | any);

        /**
         * Creates an instance of PartialMerkleTree from buffer reader
         * @param {BufferReader} bufferReader
         * @return {PartialMerkleTree}
         */
        static fromBufferReader(bufferReader: BufferReader): PartialMerkleTree;

        /**
         * @param {Buffer} buffer
         * @return {PartialMerkleTree}
         */
        static fromBuffer(buffer: Buffer): PartialMerkleTree;

        /**
         * @param {string} hexString
         * @return {PartialMerkleTree}
         */
        static fromHexString(hexString: string): PartialMerkleTree;

        /**
         * @return {Buffer}
         */
        toBuffer(): Buffer;

        /**
         * @return {PartialMerkleTree}
         */
        copy(): PartialMerkleTree;

        /**
         * @return {string}
         */
        toString(): string;

        totalTransactions: number;
        merkleHashes: string[];
        merkleFlags: number[];
    }

    /**
     * Instantiate a Block from a Buffer, JSON object, or Object with
     * the properties of the Block
     *
     * @param {*} - A Buffer, JSON string, or Object
     * @returns {Block}
     * @constructor
     */
    export class Block {
        constructor(arg: any);

        /**
         * @param {Object} - A plain JavaScript object
         * @returns {Block} - An instance of block
         */
        static fromObject(obj: any): Block;

        /**
         * @param {BufferReader} br A buffer reader of the block
         * @returns {Block} - An instance of block
         */
        static fromBufferReader(br: BufferReader): Block;

        /**
         * @param {Buffer} buf A buffer of the block
         * @returns {Block} - An instance of block
         */
        static fromBuffer(buf: Buffer): Block;

        /**
         * @param {string} str - A hex encoded string of the block
         * @returns {Block} - A hex encoded string of the block
         */
        static fromString(str: string): Block;

        /**
         * @param {Buffer} data Raw block binary data or buffer
         * @returns {Block} - An instance of block
         */
        static fromRawBlock(data: Buffer): Block;

        /**
         * @function
         * @returns {Object} - A plain object with the block properties
         */
        toObject(): any;

        /**
         * @returns {Buffer} - A buffer of the block
         */
        toBuffer(): Buffer;

        /**
         * @returns {string} - A hex encoded string of the block
         */
        toString(): string;

        /**
         * @param {BufferWriter} - An existing instance of BufferWriter
         * @returns {BufferWriter} - An instance of BufferWriter representation of the Block
         */
        toBufferWriter(bw: BufferWriter): BufferWriter;

        /**
         * Will iterate through each transaction and return an array of hashes
         * @returns {Array} - An array with transaction hashes
         */
        getTransactionHashes(): any[];

        /**
         * Will build a merkle tree of all the transactions, ultimately arriving at
         * a single point, the merkle root.
         * @link https://en.bitcoin.it/wiki/Protocol_specification#Merkle_Trees
         * @returns {Array} - An array with each level of the tree after the other.
         */
        getMerkleTree(): any[];

        /**
         * Calculates the merkleRoot from the transactions.
         * @returns {Buffer} - A buffer of the merkle root hash
         */
        getMerkleRoot(): Buffer;

        /**
         * Verifies that the transactions in the block match the header merkle root
         * @returns {Boolean} - If the merkle roots match
         */
        validMerkleRoot(): boolean;

        /**
         * @returns {Buffer} - The little endian hash buffer of the header
         */
        _getHash(): Buffer;

        /**
         * @returns {string} - A string formatted for the console
         */
        inspect(): string;
    }

    /**
     * Instantiate a BlockHeader from a Buffer, JSON object, or Object with
     * the properties of the BlockHeader
     *
     * @param {*} - A Buffer, JSON string, or Object
     * @returns {BlockHeader} - An instance of block header
     * @constructor
     */
    export class BlockHeader {
        constructor();

        /**
         * @param {Object} - A plain JavaScript object
         * @returns {BlockHeader} - An instance of block header
         */
        static fromObject(obj: any): BlockHeader;

        /**
         * @param {Buffer|string} data Raw block binary data or buffer
         * @returns {BlockHeader} - An instance of block header
         */
        static fromRawBlock(data: Buffer | string): BlockHeader;

        /**
         * @param {Buffer} - A buffer of the block header
         * @returns {BlockHeader} - An instance of block header
         */
        static fromBuffer(buf: Buffer): BlockHeader;

        /**
         * @param {string} - A hex encoded buffer of the block header
         * @returns {BlockHeader} - An instance of block header
         */
        static fromString(str: string): BlockHeader;

        /**
         * @param {BufferReader} - A BufferReader of the block header
         * @returns {BlockHeader} - An instance of block header
         */
        static fromBufferReader(br: BufferReader): BlockHeader;

        /**
         * @function
         * @returns {Object} - A plain object of the BlockHeader
         */
        toObject(): any;

        /**
         * @returns {Buffer} - A Buffer of the BlockHeader
         */
        toBuffer(): Buffer;

        /**
         * @returns {string} - A hex encoded string of the BlockHeader
         */
        toString(): string;

        /**
         * @param {BufferWriter} - An existing instance BufferWriter
         * @returns {BufferWriter} - An instance of BufferWriter representation of the BlockHeader
         */
        toBufferWriter(bw: BufferWriter): BufferWriter;

        /**
         * Returns the target difficulty for this block
         * @param {Number} bits
         * @returns {BN} An instance of BN with the decoded difficulty bits
         */
        getTargetDifficulty(bits: number): BN;

        /**
         * @link https://en.bitcoin.it/wiki/Difficulty
         * @return {Number}
         */
        getDifficulty(): number;

        /**
         * @returns {Buffer} - The little endian hash buffer of the header
         */
        _getHash(): Buffer;

        /**
         * @returns {Boolean} - If timestamp is not too far in the future
         */
        validTimestamp(): boolean;

        /**
         * @returns {Boolean} - If the proof-of-work hash satisfies the target difficulty
         */
        validProofOfWork(): boolean;

        /**
         * @returns {string} - A string formatted for the console
         */
        inspect(): string;
    }

    /**
     * Instantiate a MerkleBlock from a Buffer, JSON object, or Object with
     * the properties of the Block
     *
     * @param {Buffer|string|{
     *    header: BlockHeader|Object,
     *    numTransactions: number,
     *    hashes: string[],
     *    flags: number[]
     *  }} arg A Buffer, JSON string, or Object representing a MerkleBlock
     * @returns {MerkleBlock}
     * @constructor
     */
    export class MerkleBlock {
        constructor(arg: Buffer | string | any);

        /**
         * @name MerkleBlock#header
         * @type {BlockHeader}
         */
        header: BlockHeader;
        /**
         * @name MerkleBlock#numTransactions
         * @type {Number}
         */
        numTransactions: number;
        /**
         * @name MerkleBlock#hashes
         * @type {String[]}
         */
        hashes: String[];
        /**
         * @name MerkleBlock#flags
         * @type {Number[]}
         */
        flags: Number[];

        /**
         * Builds merkle block from block header, transaction hashes and filter matches
         * @param {BlockHeader|Object} header
         * @param {Buffer[]} transactionHashes
         * @param {boolean[]} filterMatches
         * @return {MerkleBlock}
         */
        static build(header: BlockHeader | any, transactionHashes: Buffer[], filterMatches: boolean[]): MerkleBlock;

        /**
         * @param {Buffer} - MerkleBlock data in a Buffer object
         * @returns {MerkleBlock} - A MerkleBlock object
         */
        static fromBuffer(buf: Buffer): MerkleBlock;

        /**
         * @param {BufferReader} - MerkleBlock data in a BufferReader object
         * @returns {MerkleBlock} - A MerkleBlock object
         */
        static fromBufferReader(br: BufferReader): MerkleBlock;

        /**
         * @returns {Buffer} - A buffer of the block
         */
        toBuffer(): Buffer;

        /**
         * @param {BufferWriter} - An existing instance of BufferWriter
         * @returns {BufferWriter} - An instance of BufferWriter representation of the MerkleBlock
         */
        toBufferWriter(bw: BufferWriter): BufferWriter;

        /**
         * @function
         * @returns {Object} - A plain object with the MerkleBlock properties
         */
        toObject(): any;

        /**
         * Verify that the MerkleBlock is valid
         * @returns {Boolean} - True/False whether this MerkleBlock is Valid
         */
        validMerkleTree(): boolean;

        /**
         * @return {string[]}
         */
        getMatchedTransactionHashes(): string[];

        /**
         * @param {Object} - A plain JavaScript object
         * @returns {Block} - An instance of block
         */
        static fromObject(obj: any): Block;
    }

    /**
     * @class BN
     */
    export class BN {
    }

    /**
     *
     * Instantiate a valid secp256k1 Point from the X and Y coordinates.
     *
     * @param {BN|String} x - The X coordinate
     * @param {BN|String} y - The Y coordinate
     * @link https://github.com/indutny/elliptic
     * @augments elliptic.curve.point
     * @throws {Error} A validation error if exists
     * @returns {Point} An instance of Point
     * @constructor
     */
// @ts-ignore
    export class Point extends elliptic.curve.point {
        constructor(x: BN | string, y: BN | string);

        /**
         *
         * Instantiate a valid secp256k1 Point from only the X coordinate
         *
         * @param {boolean} odd - If the Y coordinate is odd
         * @param {BN|String} x - The X coordinate
         * @throws {Error} A validation error if exists
         * @returns {Point} An instance of Point
         */
        static fromX(odd: boolean, x: BN | string): Point;

        /**
         *
         * Will return a secp256k1 ECDSA base point.
         *
         * @link https://en.bitcoin.it/wiki/Secp256k1
         * @returns {Point} An instance of the base point.
         */
        static getG(): Point;

        /**
         *
         * Will return the max of range of valid private keys as governed by the secp256k1 ECDSA standard.
         *
         * @link https://en.bitcoin.it/wiki/Private_key#Range_of_valid_ECDSA_private_keys
         * @returns {BN} A BN instance of the number of points on the curve
         */
        static getN(): BN;

        /**
         *
         * Will return the X coordinate of the Point
         *
         * @returns {BN} A BN instance of the X coordinate
         */
        getX(): BN;

        /**
         *
         * Will return the Y coordinate of the Point
         *
         * @returns {BN} A BN instance of the Y coordinate
         */
        getY(): BN;

        /**
         *
         * Will determine if the point is valid
         *
         * @link https://www.iacr.org/archive/pkc2003/25670211/25670211.pdf
         * @param {Point} An instance of Point
         * @throws {Error} A validation error if exists
         * @returns {Point} An instance of the same Point
         */
        validate(An: Point): Point;
    }

    /**
     * @param r
     * @param s
     * @returns {Signature}
     * @constructor
     */
    export class Signature {
        constructor(r: any, s: any);

        /**
         * In order to mimic the non-strict DER encoding of OpenSSL, set strict = false.
         */
        static parseDER(): void;

        /**
         * This function is translated from bitcoind's IsDERSignature and is used in
         * the script interpreter.  This "DER" format actually includes an extra byte,
         * the nhashtype, at the end. It is really the tx format, not DER format.
         *
         * A canonical signature exists of: [30] [total len] [02] [len R] [R] [02] [len S] [S] [hashtype]
         * Where R and S are not negative (their first byte has its highest bit not set), and not
         * excessively padded (do not start with a 0 byte, unless an otherwise negative number follows,
         * in which case a single 0 byte is necessary and even required).
         *
         * See https://bitcointalk.org/index.php?topic=8392.msg127623#msg127623
         */
        static isTxDER(): void;

        /**
         * Compares to bitcoind's IsLowDERSignature
         * See also ECDSA signature algorithm which enforces this.
         * See also BIP 62, "low S values in signatures"
         */
        hasLowS(): void;

        /**
         * @returns true if the nhashtype is exactly equal to one of the standard options or combinations thereof.
         * Translated from bitcoind's IsDefinedHashtypeSignature
         */
        hasDefinedHashtype(): any;
    }

    /**
     * Note that this property contains ALL masternodes, including banned ones.
     * Use getValidMasternodesList() method to get the list of only valid nodes.
     * This in needed for merkleRootNMList calculation
     * @type {SimplifiedMNListEntry[]}
     */
    export var mnList: SimplifiedMNListEntry[];

    /**
     * This property contains only valid, not PoSe-banned nodes.
     * @type {SimplifiedMNListEntry[]}
     */
    export var validMNs: SimplifiedMNListEntry[];

    /**
     * @param {Buffer|Object|string} [arg] - A Buffer, JSON string, or Object representing a MnListDiff
     * @param {string} [network]
     * @class SimplifiedMNListDiff
     * @property {string} baseBlockHash - sha256
     * @property {string} blockHash - sha256
     * @property {PartialMerkleTree} cbTxMerkleTree
     * @property {Transaction} cbTx
     * @property {Array<string>} deletedMNs - sha256 hashes of deleted MNs
     * @property {Array<SimplifiedMNListEntry>} mnList
     * @property {string} merkleRootMNList - merkle root of the whole mn list
     */
    export class SimplifiedMNListDiff {
        constructor(arg?: Buffer | any | string, network?: string);

        /**
         * Creates MnListDiff from a Buffer.
         * @param {Buffer} buffer
         * @param {string} [network]
         * @return {SimplifiedMNListDiff}
         */
        static fromBuffer(buffer: Buffer, network?: string): SimplifiedMNListDiff;

        /**
         * @param {string} hexString
         * @param {string} [network]
         * @return {SimplifiedMNListDiff}
         */
        static fromHexString(hexString: string, network?: string): SimplifiedMNListDiff;

        /**
         * Serializes mnlist diff to a Buffer
         * @return {Buffer}
         */
        toBuffer(): Buffer;

        /**
         * Creates MNListDiff from object
         * @param obj
         * @param {string} [network]
         * @return {SimplifiedMNListDiff}
         */
        static fromObject(obj: any, network?: string): SimplifiedMNListDiff;

        /**
         * sha256
         */
        baseBlockHash: string;
        /**
         * sha256
         */
        blockHash: string;
        cbTxMerkleTree: PartialMerkleTree;
        cbTx: Transaction;
        /**
         * sha256 hashes of deleted MNs
         */
        deletedMNs: string[];
        mnList: SimplifiedMNListEntry[];
        /**
         * merkle root of the whole mn list
         */
        merkleRootMNList: string;
    }

    /**
     * @typedef {Object} SMLEntry
     * @property {string} proRegTxHash
     * @property {string} confirmedHash
     * @property {string} service - ip and port
     * @property {string} pubKeyOperator - operator public key
     * @property {string} votingAddress
     * @property {boolean} isValid
     */
    export type SMLEntry = {
        proRegTxHash: string;
        confirmedHash: string;
        service: string;
        pubKeyOperator: string;
        votingAddress: string;
        isValid: boolean;
    };

    /**
     * @class SimplifiedMNListEntry
     * @param {string|Object|Buffer} arg - A Buffer, JSON string, or Object representing a SmlEntry
     * @param {string} [network]
     * @constructor
     * @property {string} proRegTxHash
     * @property {string} confirmedHash
     * @property {string} service - ip and port
     * @property {string} pubKeyOperator - operator public key
     * @property {string} votingAddress
     * @property {boolean} isValid
     */
    export class SimplifiedMNListEntry {
        constructor(arg: string | any | Buffer, network?: string);

        /**
         * Parse buffer and returns SimplifiedMNListEntry
         * @param {Buffer} buffer
         * @param {string} [network]
         * @return {SimplifiedMNListEntry}
         */
        static fromBuffer(buffer: Buffer, network?: string): SimplifiedMNListEntry;

        /**
         * @param {string} string
         * @param {string} [network]
         * @return {SimplifiedMNListEntry}
         */
        static fromHexString(string: string, network?: string): SimplifiedMNListEntry;

        /**
         * Serialize SML entry to buffer
         * @return {Buffer}
         */
        toBuffer(): Buffer;

        /**
         * Create SMLEntry from an object
         * @param {SMLEntry} obj
         * @return {SimplifiedMNListEntry}
         */
        static fromObject(obj: SMLEntry): SimplifiedMNListEntry;

        /**
         * @return {Buffer}
         */
        calculateHash(): Buffer;

        /**
         * Gets the ip from the service property
         * @return {string}
         */
        getIp(): string;

        /**
         * Creates a copy of SimplifiedMNListEntry
         * @return {SimplifiedMNListEntry}
         */
        copy(): SimplifiedMNListEntry;

        proRegTxHash: string;
        confirmedHash: string;
        /**
         * ip and port
         */
        service: string;
        /**
         * operator public key
         */
        pubKeyOperator: string;
        votingAddress: string;
        isValid: boolean;
    }

    /**
     *
     * @param buf
     * @returns {BufferReader}
     * @constructor
     */
    export class BufferReader {
        constructor(buf: any);

        /**
         * reads a length prepended buffer
         */
        readVarLengthBuffer(): void;
    }

    /**
     *
     * @param obj
     * @returns {BufferWriter}
     * @constructor
     */
    export class BufferWriter {
        constructor(obj: any);
    }

    /**
     * @class bitcore
     */
    export class bitcore {
    }

    export namespace bitcore {
        /**
         * @constructor
         */
        class Error {
        }
    }

    /**
     * Represents a generic Governance Object
     *
     * @param serialized
     * @returns {*}
     * @constructor
     */
    export class GovObject {
        constructor(serialized: any);

        /**
         * dataHex will output GovObject 'data-hex' value, should be overridden by specific object type
         *
         */
        dataHex(): void;

        /**
         * GovObject instantation method from JSON object, should be overridden by specific GovObject type
         *
         * @param arg
         * @returns Casted GovObject
         */
        fromObject(arg: any): any;

        /**
         * GovObject instantiation method from hex string
         *
         * @param string
         */
        fromString(string: any): void;

        /**
         * Retrieve a hex string that can be used with dashd's CLI interface
         *
         * @param {Object} opts allows to skip certain tests. {@see Transaction#serialize}
         * @return {string}
         */
        checkedSerialize(opts: any): string;
    }

    /**
     * Represents 'proposal' Governance Object
     *
     * @constructor
     */
    export class Proposal {
        constructor();
    }

    /**
     * Represents 'trigger' Governance Object
     *
     * @constructor
     */
    export class Trigger {
        constructor();
    }

    /**
     * Represents an instance of an hierarchically derived private key.
     *
     * More info on https://github.com/bitcoin/bips/blob/master/bip-0032.mediawiki
     *
     * @constructor
     * @param {string|Buffer|Object} arg
     */
    export class HDPrivateKey {
        constructor(arg: string | Buffer | any);

        /**
         * Verifies that a given path is valid.
         *
         * @param {string|number} arg
         * @param {boolean?} hardened
         * @return {boolean}
         */
        static isValidPath(arg: string | number, hardened: boolean): boolean;

        /**
         * Internal function that splits a string path into a derivation index array.
         * It will return null if the string path is malformed.
         * It does not validate if indexes are in bounds.
         *
         * @param {string} path
         * @return {Array}
         */
        static _getDerivationIndexes(path: string): any[];

        /**
         * WARNING: This method is deprecated. Use deriveChild or deriveNonCompliantChild instead. This is not BIP32 compliant
         *
         *
         * Get a derived child based on a string or number.
         *
         * If the first argument is a string, it's parsed as the full path of
         * derivation. Valid values for this argument include "m" (which returns the
         * same private key), "m/0/1/40/2'/1000", where the ' quote means a hardened
         * derivation.
         *
         * If the first argument is a number, the child with that index will be
         * derived. If the second argument is truthy, the hardened version will be
         * derived. See the example usage for clarification.
         *
         * @example
         * ```javascript
         * var parent = new HDPrivateKey('xprv...');
         * var child_0_1_2h = parent.derive(0).derive(1).derive(2, true);
         * var copy_of_child_0_1_2h = parent.derive("m/0/1/2'");
         * assert(child_0_1_2h.xprivkey === copy_of_child_0_1_2h);
         * ```
         *
         * @param {string|number} arg
         * @param {boolean?} hardened
         */
        derive(arg: string | number, hardened: boolean): void;

        /**
         * WARNING: This method will not be officially supported until v1.0.0.
         *
         *
         * Get a derived child based on a string or number.
         *
         * If the first argument is a string, it's parsed as the full path of
         * derivation. Valid values for this argument include "m" (which returns the
         * same private key), "m/0/1/40/2'/1000", where the ' quote means a hardened
         * derivation.
         *
         * If the first argument is a number, the child with that index will be
         * derived. If the second argument is truthy, the hardened version will be
         * derived. See the example usage for clarification.
         *
         * WARNING: The `nonCompliant` option should NOT be used, except for older implementation
         * that used a derivation strategy that used a non-zero padded private key.
         *
         * @example
         * ```javascript
         * var parent = new HDPrivateKey('xprv...');
         * var child_0_1_2h = parent.deriveChild(0).deriveChild(1).deriveChild(2, true);
         * var copy_of_child_0_1_2h = parent.deriveChild("m/0/1/2'");
         * assert(child_0_1_2h.xprivkey === copy_of_child_0_1_2h);
         * ```
         *
         * @param {string|number} arg
         * @param {boolean?} hardened
         */
        deriveChild(arg: string | number, hardened: boolean): void;

        /**
         * WARNING: This method will not be officially supported until v1.0.0
         *
         *
         * WARNING: If this is a new implementation you should NOT use this method, you should be using
         * `derive` instead.
         *
         * This method is explicitly for use and compatibility with an implementation that
         * was not compliant with BIP32 regarding the derivation algorithm. The private key
         * must be 32 bytes hashing, and this implementation will use the non-zero padded
         * serialization of a private key, such that it's still possible to derive the privateKey
         * to recover those funds.
         *
         * @param {string|number} arg
         * @param {boolean?} hardened
         */
        deriveNonCompliantChild(arg: string | number, hardened: boolean): void;

        /**
         * Verifies that a given serialized private key in base58 with checksum format
         * is valid.
         *
         * @param {string|Buffer} data - the serialized private key
         * @param {string|Network=} network - optional, if present, checks that the
         *     network provided matches the network serialized.
         * @return {boolean}
         */
        static isValidSerialized(data: string | Buffer, network: string | Network): boolean;

        /**
         * Checks what's the error that causes the validation of a serialized private key
         * in base58 with checksum to fail.
         *
         * @param {string|Buffer} data - the serialized private key
         * @param {string|Network=} network - optional, if present, checks that the
         *     network provided matches the network serialized.
         * @return {InvalidArgument|null}
         */
        // @ts-ignore
        static getSerializedError(data: string | Buffer, network: string | Network): InvalidArgument | null;

        /**
         * Generate a private key from a seed, as described in BIP32
         *
         * @param {string|Buffer} hexa
         * @param {*} network
         * @return {HDPrivateKey}
         */
        static fromSeed(hexa: string | Buffer, network: any): HDPrivateKey;

        /**
         * Receives a object with buffers in all the properties and populates the
         * internal structure
         *
         * @param {Object} arg
         * @param {Buffer} arg.version
         * @param {Buffer} arg.depth
         * @param {Buffer} arg.parentFingerPrint
         * @param {Buffer} arg.childIndex
         * @param {Buffer} arg.chainCode
         * @param {Buffer} arg.privateKey
         * @param {Buffer} arg.checksum
         * @param {string=} arg.xprivkey - if set, don't recalculate the base58
         *      representation
         * @return {HDPrivateKey} this
         */
        _buildFromBuffers(arg: {
            version: Buffer;
            depth: Buffer;
            parentFingerPrint: Buffer;
            childIndex: Buffer;
            chainCode: Buffer;
            privateKey: Buffer;
            checksum: Buffer;
            xprivkey?: string;
        }): HDPrivateKey;

        /**
         * Returns the string representation of this private key (a string starting
         * with "xprv..."
         *
         * @return {string}
         */
        toString(): string;

        /**
         * Returns the console representation of this extended private key.
         * @return {string}
         */
        inspect(): string;

        /**
         * Returns a plain object with a representation of this private key.
         *
         * Fields include:<ul>
         * <li> network: either 'livenet' or 'testnet'
         * <li> depth: a number ranging from 0 to 255
         * <li> fingerPrint: a number ranging from 0 to 2^32-1, taken from the hash of the
         * <li>     associated public key
         * <li> parentFingerPrint: a number ranging from 0 to 2^32-1, taken from the hash
         * <li>     of this parent's associated public key or zero.
         * <li> childIndex: the index from which this child was derived (or zero)
         * <li> chainCode: an hexa string representing a number used in the derivation
         * <li> privateKey: the private key associated, in hexa representation
         * <li> xprivkey: the representation of this extended private key in checksum
         * <li>     base58 format
         * <li> checksum: the base58 checksum of xprivkey
         * </ul>
         * @function
         * @return {Object}
         */
        toObject(): any;

        /**
         * Build a HDPrivateKey from a buffer
         *
         * @param {Buffer} arg
         * @return {HDPrivateKey}
         */
        static fromBuffer(arg: Buffer): HDPrivateKey;

        /**
         * Returns a buffer representation of the HDPrivateKey
         *
         * @return {string}
         */
        toBuffer(): string;
    }

    /**
     * The representation of an hierarchically derived public key.
     *
     * See https://github.com/bitcoin/bips/blob/master/bip-0032.mediawiki
     *
     * @constructor
     * @param {Object|string|Buffer} arg
     */
    export class HDPublicKey {
        constructor(arg: any | string | Buffer);

        /**
         * Verifies that a given path is valid.
         *
         * @param {string|number} arg
         * @return {boolean}
         */
        static isValidPath(arg: string | number): boolean;

        /**
         * WARNING: This method is deprecated. Use deriveChild instead.
         *
         *
         * Get a derivated child based on a string or number.
         *
         * If the first argument is a string, it's parsed as the full path of
         * derivation. Valid values for this argument include "m" (which returns the
         * same public key), "m/0/1/40/2/1000".
         *
         * Note that hardened keys can't be derived from a public extended key.
         *
         * If the first argument is a number, the child with that index will be
         * derived. See the example usage for clarification.
         *
         * @example
         * ```javascript
         * var parent = new HDPublicKey('xpub...');
         * var child_0_1_2 = parent.derive(0).derive(1).derive(2);
         * var copy_of_child_0_1_2 = parent.derive("m/0/1/2");
         * assert(child_0_1_2.xprivkey === copy_of_child_0_1_2);
         * ```
         *
         * @param {string|number} arg
         */
        derive(arg: string | number): void;

        /**
         * WARNING: This method will not be officially supported until v1.0.0.
         *
         *
         * Get a derivated child based on a string or number.
         *
         * If the first argument is a string, it's parsed as the full path of
         * derivation. Valid values for this argument include "m" (which returns the
         * same public key), "m/0/1/40/2/1000".
         *
         * Note that hardened keys can't be derived from a public extended key.
         *
         * If the first argument is a number, the child with that index will be
         * derived. See the example usage for clarification.
         *
         * @example
         * ```javascript
         * var parent = new HDPublicKey('xpub...');
         * var child_0_1_2 = parent.deriveChild(0).deriveChild(1).deriveChild(2);
         * var copy_of_child_0_1_2 = parent.deriveChild("m/0/1/2");
         * assert(child_0_1_2.xprivkey === copy_of_child_0_1_2);
         * ```
         *
         * @param {string|number} arg
         */
        deriveChild(arg: string | number): void;

        /**
         * Verifies that a given serialized public key in base58 with checksum format
         * is valid.
         *
         * @param {string|Buffer} data - the serialized public key
         * @param {string|Network=} network - optional, if present, checks that the
         *     network provided matches the network serialized.
         * @return {boolean}
         */
        static isValidSerialized(data: string | Buffer, network: string | Network): boolean;

        /**
         * Checks what's the error that causes the validation of a serialized public key
         * in base58 with checksum to fail.
         *
         * @param {string|Buffer} data - the serialized public key
         * @param {string|Network=} network - optional, if present, checks that the
         *     network provided matches the network serialized.
         * @return {bitcore.Error|null}
         */
        static getSerializedError(data: string | Buffer, network: string | Network): bitcore.Error | null;

        /**
         * Receives a object with buffers in all the properties and populates the
         * internal structure
         *
         * @param {Object} arg
         * @param {Buffer} arg.version
         * @param {Buffer} arg.depth
         * @param {Buffer} arg.parentFingerPrint
         * @param {Buffer} arg.childIndex
         * @param {Buffer} arg.chainCode
         * @param {Buffer} arg.publicKey
         * @param {Buffer} arg.checksum
         * @param {string=} arg.xpubkey - if set, don't recalculate the base58
         *      representation
         * @return {HDPublicKey} this
         */
        _buildFromBuffers(arg: {
            version: Buffer;
            depth: Buffer;
            parentFingerPrint: Buffer;
            childIndex: Buffer;
            chainCode: Buffer;
            publicKey: Buffer;
            checksum: Buffer;
            xpubkey?: string;
        }): HDPublicKey;

        /**
         * Returns the base58 checked representation of the public key
         * @return {string} a string starting with "xpub..." in livenet
         */
        toString(): string;

        /**
         * Returns the console representation of this extended public key.
         * @return {string}
         */
        inspect(): string;

        /**
         * Returns a plain JavaScript object with information to reconstruct a key.
         *
         * Fields are: <ul>
         *  <li> network: 'livenet' or 'testnet'
         *  <li> depth: a number from 0 to 255, the depth to the master extended key
         *  <li> fingerPrint: a number of 32 bits taken from the hash of the public key
         *  <li> fingerPrint: a number of 32 bits taken from the hash of this key's
         *  <li>     parent's public key
         *  <li> childIndex: index with which this key was derived
         *  <li> chainCode: string in hexa encoding used for derivation
         *  <li> publicKey: string, hexa encoded, in compressed key format
         *  <li> checksum: BufferUtil.integerFromBuffer(this._buffers.checksum),
         *  <li> xpubkey: the string with the base58 representation of this extended key
         *  <li> checksum: the base58 checksum of xpubkey
         * </ul>
         *
         * returns {object}
         */
        toObject: any;

        /**
         * Create a HDPublicKey from a buffer argument
         *
         * @param {Buffer} arg
         * @return {HDPublicKey}
         */
        static fromBuffer(arg: Buffer): HDPublicKey;

        /**
         * Return a buffer representation of the xpubkey
         *
         * @return {Buffer}
         */
        toBuffer(): Buffer;
    }

    /**
     * constructs a new message to sign and verify.
     * @constructor
     * @param {String} message
     * @returns {Message}
     */
    export class Message {
        constructor(message: string);

        /**
         * Will sign a message with a given Bitcoin private key.
         *
         * @param {PrivateKey} privateKey - An instance of PrivateKey
         * @returns {String} A base64 encoded compact signature
         */
        sign(privateKey: PrivateKey): string;

        /**
         * Will return a boolean of the signature is valid for a given bitcoin address.
         * If it isn't the specific reason is accessible via the "error" member.
         *
         * @param {Address|String} bitcoinAddress - A bitcoin address
         * @param {String} signatureString - A base64 encoded compact signature
         * @returns {Boolean}
         */
        verify(bitcoinAddress: Address | string, signatureString: string): boolean;

        /**
         * Instantiate a message from a message string
         *
         * @param {String} str - A string of the message
         * @returns {Message} A new instance of a Message
         */
        static fromString(str: string): Message;

        /**
         * Instantiate a message from JSON
         *
         * @param {String} json - An JSON string or Object with keys: message
         * @returns {Message} A new instance of a Message
         */
        static fromJSON(json: string): Message;

        /**
         * @returns {Object} A plain object with the message information
         */
        toObject(): any;

        /**
         * @returns {String} A JSON representation of the message information
         */
        toJSON(): string;

        /**
         * Will return a the string representation of the message
         *
         * @returns {String} Message
         */
        toString(): string;

        /**
         * Will return a string formatted for the console
         *
         * @returns {String} Message
         */
        inspect(): string;
    }

    /**
     * This is an immutable class that represents a BIP39 Mnemonic code.
     * See BIP39 specification for more info: https://github.com/bitcoin/bips/blob/master/bip-0039.mediawiki
     * A Mnemonic code is a a group of easy to remember words used for the generation
     * of deterministic wallets. A Mnemonic can be used to generate a seed using
     * an optional passphrase, for later generate a HDPrivateKey.
     *
     * @example
     * // generate a random mnemonic
     * var mnemonic = new Mnemonic();
     * var phrase = mnemonic.phrase;
     *
     * // use a different language
     * var mnemonic = new Mnemonic(Mnemonic.Words.SPANISH);
     * var xprivkey = mnemonic.toHDPrivateKey();
     *
     * @param {*=} data - a seed, phrase, or entropy to initialize (can be skipped)
     * @param {Array=} wordlist - the wordlist to generate mnemonics from
     * @returns {Mnemonic} A new instance of Mnemonic
     * @constructor
     */
    export class Mnemonic {
        constructor(data?: any, wordlist?: any[]);

        /**
         * Will return a boolean if the mnemonic is valid
         *
         * @example
         *
         * var valid = Mnemonic.isValid('lab rescue lunch elbow recall phrase perfect donkey biology guess moment husband');
         * // true
         *
         * @param {String} mnemonic - The mnemonic string
         * @param {String} [wordlist] - The wordlist used
         * @returns {boolean}
         */
        static isValid(mnemonic: string, wordlist?: string): boolean;

        /**
         * Internal function to check if a mnemonic belongs to a wordlist.
         *
         * @param {String} mnemonic - The mnemonic string
         * @param {String} wordlist - The wordlist
         * @returns {boolean}
         */
        static _belongsToWordlist(mnemonic: string, wordlist: string): boolean;

        /**
         * Internal function to detect the wordlist used to generate the mnemonic.
         *
         * @param {String} mnemonic - The mnemonic string
         * @returns {Array} the wordlist or null
         */
        static _getDictionary(mnemonic: string): any[];

        /**
         * Will generate a seed based on the mnemonic and optional passphrase.
         *
         * @param {String} [passphrase]
         * @returns {Buffer}
         */
        toSeed(passphrase?: string): Buffer;

        /**
         * Will generate a Mnemonic object based on a seed.
         *
         * @param {Buffer} [seed]
         * @param {string} [wordlist]
         * @returns {Mnemonic}
         */
        static fromSeed(seed?: Buffer, wordlist?: string): Mnemonic;

        /**
         *
         * Generates a HD Private Key from a Mnemonic.
         * Optionally receive a passphrase and bitcoin network.
         *
         * @param {String=} [passphrase]
         * @param {Network|String|number=} [network] - The network: 'livenet' or 'testnet'
         * @returns {HDPrivateKey}
         */
        toHDPrivateKey(passphrase?: string, network?: Network | string | number): HDPrivateKey;

        /**
         * Will return a the string representation of the mnemonic
         *
         * @returns {String} Mnemonic
         */
        toString(): string;

        /**
         * Will return a string formatted for the console
         *
         * @returns {String} Mnemonic
         */
        inspect(): string;

        /**
         * Internal function to generate a random mnemonic
         *
         * @param {Number} ENT - Entropy size, defaults to 128
         * @param {Array} wordlist - Array of words to generate the mnemonic
         * @returns {String} Mnemonic string
         */
        static _mnemonic(ENT: number, wordlist: any[]): string;

        /**
         * Internal function to generate mnemonic based on entropy
         *
         * @param {Number} entropy - Entropy buffer
         * @param {Array} wordlist - Array of words to generate the mnemonic
         * @returns {String} Mnemonic string
         */
        static _entropy2mnemonic(entropy: number, wordlist: any[]): string;
    }

    /**
     * PDKBF2
     * Credit to: https://github.com/stayradiated/pbkdf2-sha512
     * Copyright (c) 2014, JP Richardson Copyright (c) 2010-2011 Intalio Pte, All Rights Reserved
     */
    export function pbkdf2(): void;

    /**
     * A network is merely a map containing values that correspond to version
     * numbers for each bitcoin network. Currently only supporting "livenet"
     * (a.k.a. "mainnet") and "testnet".
     * @constructor
     */
    export class Network {
        constructor();
    }

    /**
     * @namespace Networks
     */
    export namespace Networks {
        /**
         * @function
         * @member Networks#get
         * Retrieves the network associated with a magic number or string.
         * @param {string|number|Network} arg
         * @param {string|Array} keys - if set, only check if the magic number associated with this name matches
         * @returns {Network}
         */
        var get: any;
        /**
         * @function
         * @member Networks#add
         * Will add a custom Network
         * @param {Object} data
         * @param {string} data.name - The name of the network
         * @param {string} data.alias - The aliased name of the network
         * @param {Number} data.pubkeyhash - The publickey hash prefix
         * @param {Number} data.privatekey - The privatekey prefix
         * @param {Number} data.scripthash - The scripthash prefix
         * @param {Number} data.xpubkey - The extended public key magic
         * @param {Number} data.xprivkey - The extended private key magic
         * @param {Number} data.networkMagic - The network magic number
         * @param {Number} data.port - The network port
         * @param {Array}  data.dnsSeeds - An array of dns seeds
         * @return {Network}
         */
        var add: any;
        /**
         * @function
         * @member Networks#remove
         * Will remove a custom network
         * @param {Network} network
         */
        var remove: any;
        /**
         * @instance
         * @member Networks#livenet
         */
        var livenet: any;
        /**
         * @instance
         * @member Networks#testnet
         */
        var testnet: any;
        /**
         * @function
         * @member Networks#enableRegtest
         * Will enable regtest features for testnet
         */
        var enableRegtest: any;
        /**
         * @function
         * @member Networks#disableRegtest
         * Will disable regtest features for testnet
         */
        var disableRegtest: any;
    }

    /**
     * Instantiate a PrivateKey from a BN, Buffer and WIF.
     *
     * @example
     * ```javascript
     * // generate a new random key
     * var key = PrivateKey();
     *
     * // get the associated address
     * var address = key.toAddress();
     *
     * // encode into wallet export format
     * var exported = key.toWIF();
     *
     * // instantiate from the exported (and saved) private key
     * var imported = PrivateKey.fromWIF(exported);
     * ```
     *
     * @param {string} data - The encoded data in various formats
     * @param {Network|string=} network - a {@link Network} object, or a string with the network name
     * @returns {PrivateKey} A new valid instance of a PrivateKey
     * @constructor
     */
    export class PrivateKey {
        constructor(data: string, network: Network | string);

        /**
         * Internal helper to instantiate PrivateKey internal `info` object from
         * different kinds of arguments passed to the constructor.
         *
         * @param {*} data
         * @param {Network|string=} network - a {@link Network} object, or a string with the network name
         * @return {Object}
         */
        _classifyArguments(data: any, network: Network | string): any;

        /**
         * Instantiate a PrivateKey from a Buffer with the DER or WIF representation
         *
         * @param {Buffer} arg
         * @param {Network} network
         * @return {PrivateKey}
         */
        static fromBuffer(arg: Buffer, network: Network): PrivateKey;

        /**
         * Instantiate a PrivateKey from a WIF string
         *
         * @function
         * @param {string} str - The WIF encoded private key string
         * @returns {PrivateKey} A new valid instance of PrivateKey
         */
        static fromString(str: string): PrivateKey;

        /**
         * Instantiate a PrivateKey from a plain JavaScript object
         *
         * @param {Object} obj - The output from privateKey.toObject()
         */
        static fromObject(obj: any): void;

        /**
         * Instantiate a PrivateKey from random bytes
         *
         * @param {string=} network - Either "livenet" or "testnet"
         * @returns {PrivateKey} A new valid instance of PrivateKey
         */
        static fromRandom(network?: string): PrivateKey;

        /**
         * Check if there would be any errors when initializing a PrivateKey
         *
         * @param {string} data - The encoded data in various formats
         * @param {string=} network - Either "livenet" or "testnet"
         * @returns {null|Error} An error if exists
         */
        static getValidationError(data: string, network?: string): null | Error;

        /**
         * Check if the parameters are valid
         *
         * @param {string} data - The encoded data in various formats
         * @param {string=} network - Either "livenet" or "testnet"
         * @returns {Boolean} If the private key is would be valid
         */
        static isValid(data: string, network?: string): boolean;

        /**
         * Will output the PrivateKey encoded as hex string
         *
         * @returns {string}
         */
        toString(): string;

        /**
         * Will output the PrivateKey to a WIF string
         *
         * @returns {string} A WIP representation of the private key
         */
        toWIF(): string;

        /**
         * Will return the private key as a BN instance
         *
         * @returns {BN} A BN instance of the private key
         */
        toBigNumber(): BN;

        /**
         * Will return the private key as a BN buffer
         *
         * @returns {Buffer} A buffer of the private key
         */
        toBuffer(): Buffer;

        /**
         * WARNING: This method will not be officially supported until v1.0.0.
         *
         *
         * Will return the private key as a BN buffer without leading zero padding
         *
         * @returns {Buffer} A buffer of the private key
         */
        toBufferNoPadding(): Buffer;

        /**
         * Will return the corresponding public key
         *
         * @returns {PublicKey} A public key generated from the private key
         */
        toPublicKey(): PublicKey;

        /**
         * Will return an address for the private key
         * @param {Network=} network - optional parameter specifying
         * the desired network for the address
         *
         * @returns {Address} An address generated from the private key
         */
        toAddress(network?: Network): Address;

        /**
         * @function
         * @returns {Object} A plain object representation
         */
        toObject(): any;

        /**
         * Will return a string formatted for the console
         *
         * @returns {string} Private key
         */
        inspect(): string;
    }

    /**
     * Instantiate a PublicKey from a {@link PrivateKey}, {@link Point}, `string`, or `Buffer`.
     *
     * There are two internal properties, `network` and `compressed`, that deal with importing
     * a PublicKey from a PrivateKey in WIF format. More details described on {@link PrivateKey}
     *
     * @example
     * ```javascript
     * // instantiate from a private key
     * var key = PublicKey(privateKey, true);
     *
     * // export to as a DER hex encoded string
     * var exported = key.toString();
     *
     * // import the public key
     * var imported = PublicKey.fromString(exported);
     * ```
     *
     * @param {string} data - The encoded data in various formats
     * @param {Object} extra - additional options
     * @param {Network=} extra.network - Which network should the address for this public key be for
     * @param {String=} extra.compressed - If the public key is compressed
     * @returns {PublicKey} A new valid instance of a PublicKey
     * @constructor
     */
    export class PublicKey {
        constructor(data: string, extra: {
            network?: Network;
            compressed?: string;
        });

        /**
         * Internal function to differentiate between arguments passed to the constructor
         * @param {*} data
         * @param {Object} extra
         */
        _classifyArgs(data: any, extra: any): void;

        /**
         * Instantiate a PublicKey from a PrivateKey
         *
         * @param {PrivateKey} privkey - An instance of PrivateKey
         * @returns {PublicKey} A new valid instance of PublicKey
         */
        static fromPrivateKey(privkey: PrivateKey): PublicKey;

        /**
         * Instantiate a PublicKey from a Buffer
         *
         * @function
         * @param {Buffer} buf - A DER hex buffer
         * @param {bool=} strict - if set to false, will loosen some conditions
         * @returns {PublicKey} A new valid instance of PublicKey
         */
        static fromDER(buf: Buffer, strict?: boolean): PublicKey;

        /**
         * Instantiate a PublicKey from a Point
         *
         * @param {Point} point - A Point instance
         * @param {boolean=} compressed - whether to store this public key as compressed format
         * @returns {PublicKey} A new valid instance of PublicKey
         */
        static fromPoint(point: Point, compressed?: boolean): PublicKey;

        /**
         * Instantiate a PublicKey from a DER hex encoded string
         *
         * @param {string} str - A DER hex string
         * @param {String=} encoding - The type of string encoding
         * @returns {PublicKey} A new valid instance of PublicKey
         */
        static fromString(str: string, encoding?: string): PublicKey;

        /**
         * Instantiate a PublicKey from an X Point
         *
         * @param {Boolean} odd - If the point is above or below the x axis
         * @param {Point} x - The x point
         * @returns {PublicKey} A new valid instance of PublicKey
         */
        static fromX(odd: boolean, x: Point): PublicKey;

        /**
         * Check if there would be any errors when initializing a PublicKey
         *
         * @param {string} data - The encoded data in various formats
         * @returns {null|Error} An error if exists
         */
        static getValidationError(data: string): null | Error;

        /**
         * Check if the parameters are valid
         *
         * @param {string} data - The encoded data in various formats
         * @returns {Boolean} If the public key would be valid
         */
        static isValid(data: string): boolean;

        /**
         * @function
         * @returns {Object} A plain object of the PublicKey
         */
        toObject(): any;

        /**
         * Will output the PublicKey to a DER Buffer
         *
         * @function
         * @returns {Buffer} A DER hex encoded buffer
         */
        toBuffer(): Buffer;

        /**
         * Will return a sha256 + ripemd160 hash of the serialized public key
         * @see https://github.com/bitcoin/bitcoin/blob/master/src/pubkey.h#L141
         * @returns {Buffer}
         */
        _getID(): Buffer;

        /**
         * Will return an address for the public key
         *
         * @param {String|Network=} network - Which network should the address be for
         * @returns {Address} An address generated from the public key
         */
        toAddress(network: string | Network): Address;

        /**
         * Will output the PublicKey to a DER encoded hex string
         *
         * @returns {string} A DER hex encoded string
         */
        toString(): string;

        /**
         * Will return a string formatted for the console
         *
         * @returns {string} Public key
         */
        inspect(): string;
    }

    /**
     * Bitcoin transactions contain scripts. Each input has a script called the
     * scriptSig, and each output has a script called the scriptPubkey. To validate
     * an input, the input's script is concatenated with the referenced output script,
     * and the result is executed. If at the end of execution the stack contains a
     * "true" value, then the transaction is valid.
     *
     * The primary way to use this class is via the verify function.
     * e.g., Interpreter().verify( ... );
     */
    export function Interpreter(): void;

    /**
     * A Bitcoin transaction script. Each transaction's inputs and outputs
     * has a script that is evaluated to validate its spending.
     *
     * See https://en.bitcoin.it/wiki/Script
     *
     * @constructor
     * @param {Object|string|Buffer=} from optional data to populate script
     */
    export class Script {
        constructor(from: any | string | Buffer);

        /**
         * @returns {boolean} if this is a pay to pubkey hash output script
         */
        isPublicKeyHashOut(): boolean;

        /**
         * @returns {boolean} if this is a pay to public key hash input script
         */
        isPublicKeyHashIn(): boolean;

        /**
         * @returns {boolean} if this is a public key output script
         */
        isPublicKeyOut(): boolean;

        /**
         * @returns {boolean} if this is a pay to public key input script
         */
        isPublicKeyIn(): boolean;

        /**
         * @returns {boolean} if this is a p2sh output script
         */
        isScriptHashOut(): boolean;

        /**
         * @returns {boolean} if this is a p2sh input script
         * Note that these are frequently indistinguishable from pubkeyhashin
         */
        isScriptHashIn(): boolean;

        /**
         * @returns {boolean} if this is a mutlsig output script
         */
        isMultisigOut(): boolean;

        /**
         * @returns {boolean} if this is a multisig input script
         */
        isMultisigIn(): boolean;

        /**
         * @returns {boolean} true if this is a valid standard OP_RETURN output
         */
        isDataOut(): boolean;

        /**
         * Retrieve the associated data for this script.
         * In the case of a pay to public key hash or P2SH, return the hash.
         * In the case of a standard OP_RETURN, return the data
         * @returns {Buffer}
         */
        getData(): Buffer;

        /**
         * @returns {boolean} if the script is only composed of data pushing
         * opcodes or small int opcodes (OP_0, OP_1, ..., OP_16)
         */
        isPushOnly(): boolean;

        /**
         * @returns {object} The Script type if it is a known form,
         * or Script.UNKNOWN if it isn't
         */
        classify(): any;

        /**
         * @returns {object} The Script type if it is a known form,
         * or Script.UNKNOWN if it isn't
         */
        classifyOutput(): any;

        /**
         * @returns {object} The Script type if it is a known form,
         * or Script.UNKNOWN if it isn't
         */
        classifyInput(): any;

        /**
         * @returns {boolean} if script is one of the known types
         */
        isStandard(): boolean;

        /**
         * Adds a script element at the start of the script.
         * @param {*} obj a string, number, Opcode, Buffer, or object to add
         * @returns {Script} this script instance
         */
        prepend(obj: any): Script;

        /**
         * Compares a script with another script
         */
        equals(): void;

        /**
         * Adds a script element to the end of the script.
         *
         * @param {*} obj a string, number, Opcode, Buffer, or object to add
         * @returns {Script} this script instance
         *
         */
        add(obj: any): Script;

        /**
         * @returns {Script} a new Multisig output script for given public keys,
         * requiring m of those public keys to spend
         * @param {PublicKey[]} publicKeys - list of all public keys controlling the output
         * @param {number} threshold - amount of required signatures to spend the output
         * @param {Object=} opts - Several options:
         *        - noSorting: defaults to false, if true, don't sort the given
         *                      public keys before creating the script
         */
        static buildMultisigOut(publicKeys: PublicKey[], threshold: number, opts?: any): Script;

        /**
         * A new Multisig input script for the given public keys, requiring m of those public keys to spend
         *
         * @param {PublicKey[]} pubkeys list of all public keys controlling the output
         * @param {number} threshold amount of required signatures to spend the output
         * @param {Array} signatures and array of signature buffers to append to the script
         * @param {Object=} opts
         * @param {boolean=} opts.noSorting don't sort the given public keys before creating the script (false by default)
         * @param {Script=} opts.cachedMultisig don't recalculate the redeemScript
         *
         * @returns {Script}
         */
        static buildMultisigIn(pubkeys: PublicKey[], threshold: number, signatures: any[], opts?: {
            noSorting?: boolean;
            cachedMultisig?: Script;
        }): Script;

        /**
         * A new P2SH Multisig input script for the given public keys, requiring m of those public keys to spend
         *
         * @param {PublicKey[]} pubkeys list of all public keys controlling the output
         * @param {number} threshold amount of required signatures to spend the output
         * @param {Array} signatures and array of signature buffers to append to the script
         * @param {Object=} opts
         * @param {boolean=} opts.noSorting don't sort the given public keys before creating the script (false by default)
         * @param {Script=} opts.cachedMultisig don't recalculate the redeemScript
         *
         * @returns {Script}
         */
        static buildP2SHMultisigIn(pubkeys: PublicKey[], threshold: number, signatures: any[], opts?: {
            noSorting?: boolean;
            cachedMultisig?: Script;
        }): Script;

        /**
         * @returns {Script} a new pay to public key hash output for the given
         * address or public key
         * @param {(Address|PublicKey)} to - destination address or public key
         */
        static buildPublicKeyHashOut(to: Address | PublicKey): Script;

        /**
         * @returns {Script} a new pay to public key output for the given
         *  public key
         */
        static buildPublicKeyOut(): Script;

        /**
         * @returns {Script} a new OP_RETURN script with data
         * @param {(string|Buffer)} data - the data to embed in the output
         * @param {(string)} encoding - the type of encoding of the string
         */
        static buildDataOut(data: string | Buffer, encoding: string): Script;

        /**
         * @param {Script|Address} script - the redeemScript for the new p2sh output.
         *    It can also be a p2sh address
         * @returns {Script} new pay to script hash script for given script
         */
        static buildScriptHashOut(script: Script | Address): Script;

        /**
         * Builds a scriptSig (a script for an input) that signs a public key output script.
         *
         * @param {Signature|Buffer} signature - a Signature object, or the signature in DER canonical encoding
         * @param {number=} sigtype - the type of the signature (defaults to SIGHASH_ALL)
         */
        static buildPublicKeyIn(signature: Signature | Buffer, sigtype?: number): void;

        /**
         * Builds a scriptSig (a script for an input) that signs a public key hash
         * output script.
         *
         * @param {Buffer|string|PublicKey} publicKey
         * @param {Signature|Buffer} signature - a Signature object, or the signature in DER canonical encoding
         * @param {number=} sigtype - the type of the signature (defaults to SIGHASH_ALL)
         */
        static buildPublicKeyHashIn(publicKey: Buffer | string | PublicKey, signature: Signature | Buffer, sigtype?: number): void;

        /**
         * @returns {Script} an empty script
         */
        static empty(): Script;

        /**
         * @returns {Script} a new pay to script hash script that pays to this script
         */
        toScriptHashOut(): Script;

        /**
         * @return {Script} an output script built from the address
         */
        static fromAddress(): Script;

        /**
         * Will return the associated address information object
         * @return {Address|boolean}
         */
        getAddressInfo(): Address | boolean;

        /**
         * @param {Network=} network
         * @return {Address|boolean} the associated address for this script if possible, or false
         */
        toAddress(network?: Network): Address | boolean;

        /**
         * Analogous to bitcoind's FindAndDelete. Find and delete equivalent chunks,
         * typically used with push data chunks.  Note that this will find and delete
         * not just the same data, but the same data with the same push data op as
         * produced by default. i.e., if a pushdata in a tx does not use the minimal
         * pushdata op, then when you try to remove the data it is pushing, it will not
         * be removed, because they do not use the same pushdata op.
         */
        findAndDelete(): void;

        /**
         * Comes from bitcoind's script interpreter CheckMinimalPush function
         * @returns {boolean} if the chunk {i} is the smallest way to push that particular data.
         */
        checkMinimalPush(): boolean;

        /**
         * Comes from bitcoind's script DecodeOP_N function
         * @param {number} opcode
         * @returns {number} numeric value in range of 0 to 16
         */
        _decodeOP_N(opcode: number): number;

        /**
         * Comes from bitcoind's script GetSigOpCount(boolean) function
         * @param {boolean} use current (true) or pre-version-0.6 (false) logic
         * @returns {number} number of signature operations required by this script
         */
        getSignatureOperationsCount(use: boolean): number;
    }

    /**
     * @param params
     * @returns {Input|*}
     * @constructor
     */
    export class Input {
        constructor(params: any);

        /**
         * Retrieve signatures for the provided PrivateKey.
         *
         * @param {Transaction} transaction - the transaction to be signed
         * @param {PrivateKey} privateKey - the private key to use when signing
         * @param {number} inputIndex - the index of this input in the provided transaction
         * @param {number} sigType - defaults to Signature.SIGHASH_ALL
         * @param {Buffer} addressHash - if provided, don't calculate the hash of the
         *     public key associated with the private key provided
         * @abstract
         */
        getSignatures(transaction: Transaction, privateKey: PrivateKey, inputIndex: number, sigType: number, addressHash: Buffer): void;

        /**
         * @returns true if this is a coinbase input (represents no input)
         */
        isNull(): any;
    }

    /**
     * @constructor
     */
    export class MultiSigInput {
        constructor();

        /**
         *
         * @param {Buffer[]} signatures
         * @param {PublicKey[]} publicKeys
         * @param {Transaction} transaction
         * @param {number} inputIndex
         * @param {Input} input
         * @returns {TransactionSignature[]}
         */
        static normalizeSignatures(signatures: Buffer[], publicKeys: PublicKey[], transaction: Transaction, inputIndex: number, input: Input): TransactionSignature[];
    }

    /**
     * @constructor
     */
    export class MultiSigScriptHashInput {
        constructor();
    }

    /**
     * Represents a special kind of input of PayToPublicKey kind.
     * @constructor
     */
    export class PublicKeyInput {
        constructor();

        /**
         * @param {Transaction} transaction - the transaction to be signed
         * @param {PrivateKey} privateKey - the private key with which to sign the transaction
         * @param {number} index - the index of the input in the transaction input vector
         * @param {number=} sigtype - the type of signature, defaults to Signature.SIGHASH_ALL
         * @return {Array} of objects that can be
         */
        getSignatures(transaction: Transaction, privateKey: PrivateKey, index: number, sigtype?: number): any[];

        /**
         * Add the provided signature
         *
         * @param {Object} signature
         * @param {PublicKey} signature.publicKey
         * @param {Signature} signature.signature
         * @param {number=} signature.sigtype
         * @return {PublicKeyInput} this, for chaining
         */
        addSignature(signature: {
            publicKey: PublicKey;
            signature: Signature;
            sigtype?: number;
        }): PublicKeyInput;

        /**
         * Clear the input's signature
         * @return {PublicKeyHashInput} this, for chaining
         */
        clearSignatures(): PublicKeyHashInput;

        /**
         * Query whether the input is signed
         * @return {boolean}
         */
        isFullySigned(): boolean;
    }

    /**
     * Represents a special kind of input of PayToPublicKeyHash kind.
     * @constructor
     */
    export class PublicKeyHashInput {
        constructor();

        /**
         * @param {Transaction} transaction - the transaction to be signed
         * @param {PrivateKey} privateKey - the private key with which to sign the transaction
         * @param {number} index - the index of the input in the transaction input vector
         * @param {number=} sigtype - the type of signature, defaults to Signature.SIGHASH_ALL
         * @param {Buffer=} hashData - the precalculated hash of the public key associated with the privateKey provided
         * @return {Array} of objects that can be
         */
        getSignatures(transaction: Transaction, privateKey: PrivateKey, index: number, sigtype?: number, hashData?: Buffer): any[];

        /**
         * Add the provided signature
         *
         * @param {Object} signature
         * @param {PublicKey} signature.publicKey
         * @param {Signature} signature.signature
         * @param {number=} signature.sigtype
         * @return {PublicKeyHashInput} this, for chaining
         */
        addSignature(signature: {
            publicKey: PublicKey;
            signature: Signature;
            sigtype?: number;
        }): PublicKeyHashInput;

        /**
         * Clear the input's signature
         * @return {PublicKeyHashInput} this, for chaining
         */
        clearSignatures(): PublicKeyHashInput;

        /**
         * Query whether the input is signed
         * @return {boolean}
         */
        isFullySigned(): boolean;
    }

    /**
     * @param args
     * @returns {Output}
     * @constructor
     */
    export class Output {
        constructor(args: any);
    }

    /**
     * @constructor
     */
    export class AbstractPayload {
        constructor();

        /**
         *
         * @param [options]
         * @param {Boolean} options.skipSignature - skip signature when serializing. Needed for signing payload
         * @return {Buffer}
         */
        toBuffer(options?: {
            skipSignature: boolean;
        }): Buffer;

        /**
         * @param [options]
         * @param {Boolean} options.skipSignature - skip signature when serializing. Needed for signing payload
         * @return {Object}
         */
        toJSON(options?: {
            skipSignature: boolean;
        }): any;

        /**
         * @param [options]
         * @param {Boolean} options.skipSignature - skip signature when serializing. Needed for signing payload
         * @return {string}
         */
        toString(options?: {
            skipSignature: boolean;
        }): string;

        /**
         * @param [options]
         * @param {Boolean} options.skipSignature - skip signature when serializing. Needed for signing payload
         * @return {Buffer} - hash
         */
        getHash(options?: {
            skipSignature: boolean;
        }): Buffer;

        /**
         * Signs payload
         * @param {string|PrivateKey} privateKey
         * @return {AbstractPayload}
         */
        sign(privateKey: string | PrivateKey): AbstractPayload;

        /**
         * Verify payload signature
         * @param {string|Buffer} publicKeyId
         * @return {boolean}
         */
        verifySignature(publicKeyId: string | Buffer): boolean;
    }

    /**
     * @typedef {Object} CoinbasePayloadJSON
     * @property {number} version
     * @property {number} height
     * @property {string} merkleRootMNList
     * @property {string} merkleRootQuorums
     */
    export type CoinbasePayloadJSON = {
        version: number;
        height: number;
        merkleRootMNList: string;
        merkleRootQuorums: string;
    };

    /**
     * @class CoinbasePayload
     * @property {number} version
     * @property {number} height
     * @property {string} merkleRootMNList
     * @property {string} merkleRootQuorums
     */
    export class CoinbasePayload {
        /**
         * Parse raw transition payload
         * @param {Buffer} rawPayload
         * @return {CoinbasePayload}
         */
        static fromBuffer(rawPayload: Buffer): CoinbasePayload;

        /**
         * Create new instance of payload from JSON
         * @param {string|CoinbasePayloadJSON} payloadJson
         * @return {CoinbasePayload}
         */
        static fromJSON(payloadJson: string | CoinbasePayloadJSON): CoinbasePayload;

        /**
         * Validates payload data
         * @return {boolean}
         */
        validate(): boolean;

        /**
         * Serializes payload to JSON
         * @return {CoinbasePayloadJSON}
         */
        toJSON(): CoinbasePayloadJSON;

        /**
         * Serialize payload to buffer
         * @return {Buffer}
         */
        toBuffer(): Buffer;

        /**
         * Copy payload instance
         * @return {CoinbasePayload}
         */
        copy(): CoinbasePayload;

        version: number;
        height: number;
        merkleRootMNList: string;
        merkleRootQuorums: string;
    }

    /**
     * @typedef {Object} CommitmentTxPayloadJSON
     * @property {number} version    uint16_t    2    Commitment special transaction version number. Currently set to 1. Please note that this is not the same as the version field of qfcommit
     * @property {number} height    uint16_t    2    The height of the block in which this commitment is included
     * @property {number} qfcVersion    uint16_t    2    Version of the final commitment message
     * @property {number} llmqtype    uint8_t    1    type of the long living masternode quorum
     * @property {string} quorumHash    uint256    32    The quorum identifier
     * @property {number} signersSize    compactSize uint    1-9    Bit size of the signers bitvector
     * @property {string} signers    byte[]    (bitSize + 7) / 8    Bitset representing the aggregated signers of this final commitment
     * @property {number} validMembersSize    compactSize uint    1-9    Bit size of the validMembers bitvector
     * @property {string} validMembers    byte[]    (bitSize + 7) / 8    Bitset of valid members in this commitment
     * @property {string} quorumPublicKey    BLSPubKey    48    The quorum public key
     * @property {string} quorumVvecHash    uint256    32    The hash of the quorum verification vector
     * @property {string} quorumSig    BLSSig    96    Recovered threshold signature
     * @property {string} sig    BLSSig    96    Aggregated BLS signatures from all included commitments
     */
    export type CommitmentTxPayloadJSON = {
        version: number;
        height: number;
        qfcVersion: number;
        llmqtype: number;
        quorumHash: string;
        signersSize: number;
        signers: string;
        validMembersSize: number;
        validMembers: string;
        quorumPublicKey: string;
        quorumVvecHash: string;
        quorumSig: string;
        sig: string;
    };

    /**
     * @class CommitmentTxPayload
     * @property {number} version
     * @property {number} height
     * @property {number} qfcVersion
     * @property {number} llmqtype
     * @property {string} quorumHash
     * @property {number} signersSize
     * @property {string} signers
     * @property {number} validMembersSize
     * @property {string} validMembers
     * @property {string} quorumPublicKey
     * @property {string} quorumVvecHash
     * @property {string} quorumSig
     * @property {string} sig
     */
    export class CommitmentTxPayload {
        /**
         * Parse raw payload
         * @param {Buffer} rawPayload
         * @return {CommitmentTxPayload}
         */
        static fromBuffer(rawPayload: Buffer): CommitmentTxPayload;

        /**
         * Create new instance of payload from JSON
         * @param {string|CommitmentTxPayloadJSON} payloadJson
         * @return {CommitmentTxPayload}
         */
        static fromJSON(payloadJson: string | CommitmentTxPayloadJSON): CommitmentTxPayload;

        /**
         * Validate payload
         * @return {boolean}
         */
        validate(): boolean;

        /**
         * Serializes payload to JSON
         * @param [options]
         * @return {CommitmentTxPayload}
         */
        toJSON(options?: any): CommitmentTxPayload;

        /**
         * Serialize payload to buffer
         * @param [options]
         * @return {Buffer}
         */
        toBuffer(options?: any): Buffer;

        version: number;
        height: number;
        qfcVersion: number;
        llmqtype: number;
        quorumHash: string;
        signersSize: number;
        signers: string;
        validMembersSize: number;
        validMembers: string;
        quorumPublicKey: string;
        quorumVvecHash: string;
        quorumSig: string;
        sig: string;
    }

    /**
     *
     * @param {number} payloadType
     * @return {AbstractPayload}
     */
    export function getPayloadClass(payloadType: number): AbstractPayload;

    /**
     * Parses payload and returns instance of payload to work with
     * @param {number} payloadType
     * @param {Buffer} rawPayload
     * @return {AbstractPayload}
     */
    export function parsePayloadBuffer(payloadType: number, rawPayload: Buffer): AbstractPayload;

    /**
     * @param {Number} payloadType
     * @param {Object} payloadJson
     * @return {AbstractPayload}
     */
    export function parsePayloadJSON(payloadType: number, payloadJson: any): AbstractPayload;

    /**
     * Create an empty instance of payload class
     * @param payloadType
     * @return {AbstractPayload}
     */
    export function createPayload(payloadType: any): AbstractPayload;

    /**
     * Checks if type matches payload
     * @param {number} payloadType
     * @param {AbstractPayload} payload
     * @return {boolean}
     */
    export function isPayloadMatchesType(payloadType: number, payload: AbstractPayload): boolean;

    /**
     * Serializes payload
     * @param {AbstractPayload} payload
     * @return {Buffer}
     */
    export function serializePayloadToBuffer(payload: AbstractPayload): Buffer;

    /**
     * Serializes payload to JSON
     * @param payload
     * @return {Object}
     */
    export function serializePayloadToJSON(payload: any): any;

    /**
     * @typedef {Object} ProRegTxPayloadJSON
     * @property {number} version    uint_16    2    Provider transaction version number. Currently set to 1.
     * @property {string} collateralHash
     * @property {number} collateralIndex    uint_32    4    The collateral index.
     * @property {string} service - service address, ip and port
     * @property {string} keyIDOwner    CKeyID    20    The public key hash used for owner related signing (ProTx updates, governance voting)
     * @property {string} pubKeyOperator    BLSPubKey    48    The public key used for operational related signing (network messages, ProTx updates)
     * @property {string} keyIDVoting    CKeyID    20    The public key hash used for voting.
     * @property {number} operatorReward    uint_16    2    A value from 0 to 10000.
     * @property {string} payoutAddress
     * @property {string} inputsHash    uint256    32    Hash of all the outpoints of the transaction inputs
     * @property {number} [payloadSigSize] Size of the Signature
     * @property {string} [payloadSig] Signature of the hash of the ProTx fields. Signed with keyIDOwner
     */
    export type ProRegTxPayloadJSON = {
        version: number;
        collateralHash: string;
        collateralIndex: number;
        service: string;
        keyIDOwner: string;
        pubKeyOperator: string;
        keyIDVoting: string;
        operatorReward: number;
        payoutAddress: string;
        inputsHash: string;
        payloadSigSize?: number;
        payloadSig?: string;
    };

    /**
     * @class ProRegTxPayload
     * @property {number} version    uint_16    2    Provider transaction version number. Currently set to 1.
     * @property {number} type
     * @property {number} mode
     * @property {string} collateralHash
     * @property {number} collateralIndex    uint_32    4    The collateral index.
     * @property {string} service - service address, ip and port
     * @property {string} keyIDOwner    CKeyID    20    The public key hash used for owner related signing (ProTx updates, governance voting)
     * @property {string} pubKeyOperator    BLSPubKey    48    The public key used for operational related signing (network messages, ProTx updates)
     * @property {string} keyIDVoting    CKeyID    20    The public key hash used for voting.
     * @property {number} operatorReward    uint_16    2    A value from 0 to 10000.
     * @property {string} scriptPayout    Script    Variable    Payee script (p2pkh/p2sh)
     * @property {string} inputsHash    uint256    32    Hash of all the outpoints of the transaction inputs
     * @property {number} [payloadSigSize] Size of the Signature
     * @property {string} [payloadSig] Signature of the hash of the ProTx fields. Signed with keyIDOwner
     */
    export class ProRegTxPayload {
        /**
         * Parse raw payload
         * @param {Buffer} rawPayload
         * @return {ProRegTxPayload}
         */
        static fromBuffer(rawPayload: Buffer): ProRegTxPayload;

        /**
         * Create new instance of payload from JSON
         * @param {string|ProRegTxPayloadJSON} payloadJson
         * @return {ProRegTxPayload}
         */
        static fromJSON(payloadJson: string | ProRegTxPayloadJSON): ProRegTxPayload;

        /**
         * Validate payload
         * @return {boolean}
         */
        validate(): boolean;

        /**
         * Serializes payload to JSON
         * @param [options]
         * @param [options.skipSignature]
         * @param [options.network] - network for address serialization
         * @return {ProRegTxPayloadJSON}
         */
        toJSON(options?: {
            skipSignature?: any;
            network?: any;
        }): ProRegTxPayloadJSON;

        /**
         * Serialize payload to buffer
         * @param [options]
         * @param {Boolean} [options.skipSignature] - skip signature. Needed for signing
         * @return {Buffer}
         */
        toBuffer(options?: {
            skipSignature?: boolean;
        }): Buffer;

        /**
         * uint_16    2    Provider transaction version number. Currently set to 1.
         */
        version: number;
        type: number;
        mode: number;
        collateralHash: string;
        /**
         * uint_32    4    The collateral index.
         */
        collateralIndex: number;
        /**
         * service address, ip and port
         */
        service: string;
        /**
         * CKeyID    20    The public key hash used for owner related signing (ProTx updates, governance voting)
         */
        keyIDOwner: string;
        /**
         * BLSPubKey    48    The public key used for operational related signing (network messages, ProTx updates)
         */
        pubKeyOperator: string;
        /**
         * CKeyID    20    The public key hash used for voting.
         */
        keyIDVoting: string;
        /**
         * uint_16    2    A value from 0 to 10000.
         */
        operatorReward: number;
        /**
         * Script    Variable    Payee script (p2pkh/p2sh)
         */
        scriptPayout: string;
        /**
         * uint256    32    Hash of all the outpoints of the transaction inputs
         */
        inputsHash: string;
        /**
         * Size of the Signature
         */
        payloadSigSize?: number;
        /**
         * Signature of the hash of the ProTx fields. Signed with keyIDOwner
         */
        payloadSig?: string;
    }

    /**
     * @typedef {Object} ProUpRegTransactionPayloadJSON
     * @property {number} version
     * @property {string} proTxHash
     * @property {string} pubKeyOperator
     * @property {string} keyIDVoting
     * @property {string} payoutAddress
     * @property {string} inputsHash
     * @property {string} [payloadSig]
     */
    export type ProUpRegTransactionPayloadJSON = {
        version: number;
        proTxHash: string;
        pubKeyOperator: string;
        keyIDVoting: string;
        payoutAddress: string;
        inputsHash: string;
        payloadSig?: string;
    };

    /**
     * @class ProUpRegTxPayload
     * @property {number} version uint_16    2    Upgrade Provider Transaction version number. Currently set to 1.
     * @property {string} proTxHash uint256    32    The hash of the provider transaction
     * @property {number} mode uint_16    2    Masternode mode
     * @property {string} pubKeyOperator BLSPubKey    48    The public key hash used for operational related signing (network messages, ProTx updates)
     * @property {string} keyIDVoting CKeyID    20    The public key hash used for voting.
     * @property {number} scriptPayoutSize compactSize uint    1-9    Size of the Payee Script.
     * @property {string} scriptPayout Script    Variable    Payee script (p2pkh/p2sh)
     * @property {string} inputsHash uint256    32    Hash of all the outpoints of the transaction inputs
     * @property {number} payloadSigSize compactSize uint    1-9    Size of the Signature
     * @property {string} payloadSig vector    Variable    Signature of the hash of the ProTx fields. Signed by the Owner.
     *
     * @param {ProUpRegTransactionPayloadJSON} [payloadJSON]
     * @constructor
     */
    export class ProUpRegTxPayload {
        constructor(payloadJSON?: ProUpRegTransactionPayloadJSON);

        /**
         * Parses raw ProUpRegTxPayload payload
         * @param {Buffer} rawPayload
         * @return {ProUpRegTxPayload}
         */
        static fromBuffer(rawPayload: Buffer): ProUpRegTxPayload;

        /**
         * Creates new instance of ProUpRegTxPayload payload from JSON
         * @param {string|ProUpRegTransactionPayloadJSON} payloadJSON
         * @return {ProUpRegTxPayload}
         */
        static fromJSON(payloadJSON: string | ProUpRegTransactionPayloadJSON): ProUpRegTxPayload;

        /**
         * Validates ProUpRegTxPayload payload data
         * @return {boolean}
         */
        validate(): boolean;

        /**
         * Serializes ProUpRegTxPayload payload to JSON
         * @param [options]
         * @param [options.skipSignature] - skip signature part. Needed for creating new signature
         * @param [options.network]
         * @return {ProUpRegTransactionPayloadJSON}
         */
        toJSON(options?: {
            skipSignature?: any;
            network?: any;
        }): ProUpRegTransactionPayloadJSON;

        /**
         * Serializes ProUpRegTxPayload to buffer
         * @param [options]
         * @param {boolean} options.skipSignature - skip signature part. Needed for creating new signature
         * @return {Buffer}
         */
        toBuffer(options?: {
            skipSignature: boolean;
        }): Buffer;

        /**
         * Copy payload instance
         * @return {ProUpRegTxPayload}
         */
        copy(): ProUpRegTxPayload;

        /**
         * uint_16    2    Upgrade Provider Transaction version number. Currently set to 1.
         */
        version: number;
        /**
         * uint256    32    The hash of the provider transaction
         */
        proTxHash: string;
        /**
         * uint_16    2    Masternode mode
         */
        mode: number;
        /**
         * BLSPubKey    48    The public key hash used for operational related signing (network messages, ProTx updates)
         */
        pubKeyOperator: string;
        /**
         * CKeyID    20    The public key hash used for voting.
         */
        keyIDVoting: string;
        /**
         * compactSize uint    1-9    Size of the Payee Script.
         */
        scriptPayoutSize: number;
        /**
         * Script    Variable    Payee script (p2pkh/p2sh)
         */
        scriptPayout: string;
        /**
         * uint256    32    Hash of all the outpoints of the transaction inputs
         */
        inputsHash: string;
        /**
         * compactSize uint    1-9    Size of the Signature
         */
        payloadSigSize: number;
        /**
         * vector    Variable    Signature of the hash of the ProTx fields. Signed by the Owner.
         */
        payloadSig: string;
    }

    /**
     * @typedef {Object} ProUpRevTransactionPayloadJSON
     * @property {number} version
     * @property {string} proTxHash
     * @property {number} reason
     * @property {string} inputsHash
     * @property {string} payloadSig
     */
    export type ProUpRevTransactionPayloadJSON = {
        version: number;
        proTxHash: string;
        reason: number;
        inputsHash: string;
        payloadSig: string;
    };

    /**
     * @class ProUpRevTxPayload
     * @property {number} version uint_16    2    ProUpRevTx version number. Currently set to 1.
     * @property {string} proTxHash uint256    32    The hash of the provider transaction
     * @property {number} reason uint_16    2    The reason for revoking the key.
     * @property {string} inputsHash uint256    32    Hash of all the outpoints of the transaction inputs
     * @property {string} payloadSig BLSSig Signature of the hash of the ProTx fields. Signed by the Operator.
     */
    export class ProUpRevTxPayload {
        /**
         * Serializes ProUpRevTxPayload payload
         * @param {ProUpRevTransactionPayloadJSON} transitionPayloadJSON
         * @return {Buffer} serialized payload
         */
        static serializeJSONToBuffer(transitionPayloadJSON: ProUpRevTransactionPayloadJSON): Buffer;

        /**
         * Parses raw ProUpRevTxPayload payload
         * @param {Buffer} rawPayloadBuffer
         * @return {ProUpRevTxPayload}
         */
        static fromBuffer(rawPayloadBuffer: Buffer): ProUpRevTxPayload;

        /**
         * Creates new instance of ProUpRevTxPayload payload from JSON
         * @param {string|ProUpRevTransactionPayloadJSON} payloadJSON
         * @return {ProUpRevTxPayload}
         */
        static fromJSON(payloadJSON: string | ProUpRevTransactionPayloadJSON): ProUpRevTxPayload;

        /**
         * Validates ProUpRevTxPayload payload data
         * @return {boolean}
         */
        validate(): boolean;

        /**
         * Serializes ProUpRevTxPayload payload to JSON
         * @param [options]
         * @param {boolean} options.skipSignature - skip signature part. Needed for creating new signature
         * @return {ProUpRevTransactionPayloadJSON}
         */
        toJSON(options?: {
            skipSignature: boolean;
        }): ProUpRevTransactionPayloadJSON;

        /**
         * Serializes ProUpRevTxPayload to buffer
         * @param [options]
         * @param {boolean} options.skipSignature - skip signature part. Needed for creating new signature
         * @return {Buffer}
         */
        toBuffer(options?: {
            skipSignature: boolean;
        }): Buffer;

        /**
         * Copy payload instance
         * @return {ProUpRevTxPayload}
         */
        copy(): ProUpRevTxPayload;

        /**
         * uint_16    2    ProUpRevTx version number. Currently set to 1.
         */
        version: number;
        /**
         * uint256    32    The hash of the provider transaction
         */
        proTxHash: string;
        /**
         * uint_16    2    The reason for revoking the key.
         */
        reason: number;
        /**
         * uint256    32    Hash of all the outpoints of the transaction inputs
         */
        inputsHash: string;
        /**
         * BLSSig Signature of the hash of the ProTx fields. Signed by the Operator.
         */
        payloadSig: string;
    }

    /**
     * @typedef {Object} ProUpServTxPayloadJSON
     * @property {number} version
     * @property {string} proTxHash
     * @property {string} service - Service string, ip and port
     * @property {string} [operatorPayoutAddress]
     * @property {string} inputsHash
     * @property {string} [payloadSig]
     */
    export type ProUpServTxPayloadJSON = {
        version: number;
        proTxHash: string;
        service: string;
        operatorPayoutAddress?: string;
        inputsHash: string;
        payloadSig?: string;
    };

    /**
     * @class ProUpServTxPayload
     * @property {number} version ProUpServTx version number. Currently set to 1.
     * @property {string} proTXHash The hash of the initial ProRegTx
     * @property {string} service string - ip and port
     * @property {string} inputsHash Hash of all the outpoints of the transaction inputs
     * @property {string} [scriptOperatorPayout] Payee script (p2pkh/p2sh)
     * @property {string} [payloadSig] BLSSig Signature of the hash of the ProUpServTx fields. Signed by the Operator.
     */
    export class ProUpServTxPayload {
        /**
         * Parse raw transition payload
         * @param {Buffer} rawPayload
         * @return {ProUpServTxPayload}
         */
        static fromBuffer(rawPayload: Buffer): ProUpServTxPayload;

        /**
         * Create new instance of payload from JSON
         * @param {ProUpServTxPayloadJSON} payloadJson
         * @return {ProUpServTxPayload}
         */
        static fromJSON(payloadJson: ProUpServTxPayloadJSON): ProUpServTxPayload;

        /**
         * Validates payload data
         * @return {boolean}
         */
        validate(): boolean;

        /**
         * Serializes payload to JSON
         * @param [options]
         * @param [options.skipSignature]
         * @param [options.network] - network param for payout address serialization
         * @return {ProUpServTxPayloadJSON}
         */
        toJSON(options?: {
            skipSignature?: any;
            network?: any;
        }): ProUpServTxPayloadJSON;

        /**
         * Serialize payload to buffer
         * @param [options]
         * @param {Boolean} options.skipSignature - skip signature. Used for generating new signature
         * @return {Buffer}
         */
        toBuffer(options?: {
            skipSignature: boolean;
        }): Buffer;

        /**
         * Copy payload instance
         * @return {ProUpServTxPayload}
         */
        copy(): ProUpServTxPayload;

        /**
         * ProUpServTx version number. Currently set to 1.
         */
        version: number;
        /**
         * The hash of the initial ProRegTx
         */
        proTXHash: string;
        /**
         * string - ip and port
         */
        service: string;
        /**
         * Hash of all the outpoints of the transaction inputs
         */
        inputsHash: string;
        /**
         * Payee script (p2pkh/p2sh)
         */
        scriptOperatorPayout?: string;
        /**
         * BLSSig Signature of the hash of the ProUpServTx fields. Signed by the Operator.
         */
        payloadSig?: string;
    }

    /**
     * @typedef {Object} SubTxCloseAccountPayloadJSON
     * @property {number} version - payload version
     * @property {string} regTxHash
     * @property {string} hashPrevSubTx
     * @property {number} creditFee - fee to pay for transaction (duffs)
     * @property {number} payloadSigSize - length of the signature (payloadSig)
     * @property {string} payloadSig - Signature from either the current key or a previous key (<= ~90 days old)
     */
    export type SubTxCloseAccountPayloadJSON = {
        version: number;
        regTxHash: string;
        hashPrevSubTx: string;
        creditFee: number;
        payloadSigSize: number;
        payloadSig: string;
    };

    /**
     * @class SubTxCloseAccountPayload
     * @property {number} version - payload version
     * @property {string} regTxHash
     * @property {string} hashPrevSubTx
     * @property {number} creditFee - fee to pay for transaction (duffs)
     * @property {number} payloadSigSize - length of the signature (payloadSig)
     * @property {string} payloadSig - Signature from either the current key or a previous key (<= ~90 days old)
     */
    export class SubTxCloseAccountPayload {
        /**
         * Parse raw transition payload
         * @param {Buffer} rawPayload
         * @return {SubTxCloseAccountPayload}
         */
        static fromBuffer(rawPayload: Buffer): SubTxCloseAccountPayload;

        /**
         * Create new instance of payload from JSON
         * @param {string|SubTxCloseAccountPayloadJSON} payloadJson
         * @return {SubTxCloseAccountPayload}
         */
        static fromJSON(payloadJson: string | SubTxCloseAccountPayloadJSON): SubTxCloseAccountPayload;

        /**
         * Validates payload data
         * @return {boolean}
         */
        validate(): boolean;

        /**
         * @param {string} regTxHash
         * @return {SubTxCloseAccountPayload}
         */
        setRegTxHash(regTxHash: string): SubTxCloseAccountPayload;

        /**
         * @param {string} hashPrevSubTx
         * @return {SubTxCloseAccountPayload}
         */
        setPrevSubTxHash(hashPrevSubTx: string): SubTxCloseAccountPayload;

        /**
         * @param {number} duffs
         * @return {SubTxCloseAccountPayload}
         */
        setCreditFee(duffs: number): SubTxCloseAccountPayload;

        /**
         * Serializes payload to JSON
         * @return {{version: *, regTxHash: *, hashPrevSubTx: *, creditFee: *, payloadSigSize: *, payloadSig: *}}
         */
        toJSON(): any;

        /**
         * Serialize payload to buffer
         * @return {Buffer}
         */
        toBuffer(): Buffer;

        /**
         * payload version
         */
        version: number;
        regTxHash: string;
        hashPrevSubTx: string;
        /**
         * fee to pay for transaction (duffs)
         */
        creditFee: number;
        /**
         * length of the signature (payloadSig)
         */
        payloadSigSize: number;
        /**
         * Signature from either the current key or a previous key (<= ~90 days old)
         */
        payloadSig: string;
    }

    /**
     * @typedef {Object} BlockchainUserPayloadJSON
     * @property {number} version - payload version
     * @property {Buffer} pubKeyId
     * @property {string} userName
     * @property {string} [payloadSig]
     * @property {string} [payloadSigSize]
     */
    export type BlockchainUserPayloadJSON = {
        version: number;
        pubKeyId: Buffer;
        userName: string;
        payloadSig?: string;
        payloadSigSize?: string;
    };

    /**
     * @class SubTxRegisterPayload
     * @property {number} version - payload version
     * @property {Buffer} pubKeyId
     * @property {string} userName
     * @property {string} [payloadSig]
     * @property {string} [payloadSigSize]
     */
    export class SubTxRegisterPayload {
        /**
         * Serialize blockchain user payload
         * @param {BlockchainUserPayloadJSON} blockchainUserPayload
         * @return {Buffer} serialized payload
         */
        static serializeJSONToBuffer(blockchainUserPayload: BlockchainUserPayloadJSON): Buffer;

        /**
         * Parse raw blockchain user payload
         * @param {Buffer} rawPayload
         * @return {SubTxRegisterPayload}
         */
        static fromBuffer(rawPayload: Buffer): SubTxRegisterPayload;

        /**
         * Create new instance of payload from JSON
         * @param {string|BlockchainUserPayloadJSON} payloadJson
         * @return {SubTxRegisterPayload}
         */
        static fromJSON(payloadJson: string | BlockchainUserPayloadJSON): SubTxRegisterPayload;

        /**
         * Validate payload
         * @param {BlockchainUserPayloadJSON} blockchainUserPayload
         * @return {boolean}
         */
        static validatePayloadJSON(blockchainUserPayload: BlockchainUserPayloadJSON): boolean;

        /**
         * @param {string} userName
         * @return {SubTxRegisterPayload}
         */
        setUserName(userName: string): SubTxRegisterPayload;

        /**
         * @param {Buffer} pubKeyId
         * @return {SubTxRegisterPayload}
         */
        setPubKeyId(pubKeyId: Buffer): SubTxRegisterPayload;

        /**
         * Extracts and sets pubKeyId from private key
         * @param {string|PrivateKey} privateKey
         * @return {SubTxRegisterPayload}
         */
        setPubKeyIdFromPrivateKey(privateKey: string | PrivateKey): SubTxRegisterPayload;

        /**
         * Serializes payload to JSON
         * @param [options]
         * @param {boolean} options.skipSignature - skip signature part. Needed for creating new signature
         * @return {BlockchainUserPayloadJSON}
         */
        toJSON(options?: {
            skipSignature: boolean;
        }): BlockchainUserPayloadJSON;

        /**
         * Serialize payload to buffer
         * @param [options]
         * @param {boolean} options.skipSignature - skip signature part. Needed for creating new signature
         * @return {Buffer}
         */
        toBuffer(options?: {
            skipSignature: boolean;
        }): Buffer;

        /**
         * payload version
         */
        version: number;
        pubKeyId: Buffer;
        userName: string;
        payloadSig?: string;
        payloadSigSize?: string;
    }

    /**
     * @typedef {Object} SubTxResetKeyPayloadJSON
     * @property {number} version - payload version
     * @property {string} regTxHash
     * @property {string} hashPrevSubTx
     * @property {number} creditFee - fee to pay for transaction (duffs)
     * @property {number} newPubKeySize - length of the new public key (not present in implementation)
     * @property {Buffer} newPubKey
     * @property {number} payloadSigSize - length of the signature (payloadSig)
     * @property {string} payloadSig - signature of most recent pubkey
     */
    export type SubTxResetKeyPayloadJSON = {
        version: number;
        regTxHash: string;
        hashPrevSubTx: string;
        creditFee: number;
        newPubKeySize: number;
        newPubKey: Buffer;
        payloadSigSize: number;
        payloadSig: string;
    };

    /**
     * @class SubTxResetKeyPayload
     * @property {number} version - payload version
     * @property {string} regTxHash
     * @property {string} hashPrevSubTx
     * @property {number} creditFee - fee to pay for transaction (duffs)
     * @property {number} newPubKeySize - length of the new public key (not present in implementation)
     * @property {Buffer} newPubKey
     * @property {number} payloadSigSize - length of the signature (payloadSig)
     * @property {string} payloadSig - signature of most recent pubkey
     */
    export class SubTxResetKeyPayload {
        /**
         * Parse raw transition payload
         * @param {Buffer} rawPayload
         * @return {SubTxResetKeyPayload}
         */
        static fromBuffer(rawPayload: Buffer): SubTxResetKeyPayload;

        /**
         * Create new instance of payload from JSON
         * @param {string|SubTxResetKeyPayloadJSON} payloadJson
         * @return {SubTxResetKeyPayload}
         */
        static fromJSON(payloadJson: string | SubTxResetKeyPayloadJSON): SubTxResetKeyPayload;

        /**
         * Validates payload data
         * @return {boolean}
         */
        validate(): boolean;

        /**
         * @param {string} regTxHash
         * @return {SubTxResetKeyPayload}
         */
        setRegTxHash(regTxHash: string): SubTxResetKeyPayload;

        /**
         * @param {string} hashPrevSubTx
         * @return {SubTxResetKeyPayload}
         */
        setPrevSubTxHash(hashPrevSubTx: string): SubTxResetKeyPayload;

        /**
         * @param {number} duffs
         * @return {SubTxResetKeyPayload}
         */
        setCreditFee(duffs: number): SubTxResetKeyPayload;

        /**
         * @param {Buffer} pubKeyId
         * @return {SubTxResetKeyPayload}
         */
        setNewPubKeyId(pubKeyId: Buffer): SubTxResetKeyPayload;

        /**
         * Extracts and sets pubKeyId from private key
         * @param {string|PrivateKey} privateKey
         * @return {SubTxResetKeyPayload}
         */
        setPubKeyIdFromPrivateKey(privateKey: string | PrivateKey): SubTxResetKeyPayload;

        /**
         * Serializes payload to JSON
         * @return {{version: *, regTxHash: *, hashPrevSubTx: *, creditFee: *, newPubKeySize: *, newPubKey: *, payloadSigSize: *, payloadSig: *}}
         */
        toJSON(): any;

        /**
         * Serialize payload to buffer
         * @return {Buffer}
         */
        toBuffer(): Buffer;

        /**
         * payload version
         */
        version: number;
        regTxHash: string;
        hashPrevSubTx: string;
        /**
         * fee to pay for transaction (duffs)
         */
        creditFee: number;
        /**
         * length of the new public key (not present in implementation)
         */
        newPubKeySize: number;
        newPubKey: Buffer;
        /**
         * length of the signature (payloadSig)
         */
        payloadSigSize: number;
        /**
         * signature of most recent pubkey
         */
        payloadSig: string;
    }

    /**
     * @typedef {Object} SubTxTopupPayloadJSON
     * @property {number} version
     * @property {string} regTxHash
     */
    export type SubTxTopupPayloadJSON = {
        version: number;
        regTxHash: string;
    };

    /**
     * @class SubTxTopupPayload
     * @property {number} version
     * @property {string} regTxHash
     */
    export class SubTxTopupPayload {
        /**
         * Parse raw transition payload
         * @param {Buffer} rawPayload
         * @return {SubTxTopupPayload}
         */
        static fromBuffer(rawPayload: Buffer): SubTxTopupPayload;

        /**
         * Create new instance of payload from JSON
         * @param {string|SubTxTopupPayloadJSON} payloadJson
         * @return {SubTxTopupPayload}
         */
        static fromJSON(payloadJson: string | SubTxTopupPayloadJSON): SubTxTopupPayload;

        /**
         * Validates payload data
         * @return {boolean}
         */
        validate(): boolean;

        /**
         * Serializes payload to JSON
         * @return {SubTxTopupPayload}
         */
        toJSON(): SubTxTopupPayload;

        /**
         * Serialize payload to buffer
         * @return {Buffer}
         */
        toBuffer(): Buffer;

        /**
         * Copy payload instance
         * @return {SubTxTopupPayload}
         */
        copy(): SubTxTopupPayload;

        /**
         * @param {string} regTxHash
         * @return {SubTxTopupPayload}
         */
        setRegTxHash(regTxHash: string): SubTxTopupPayload;

        version: number;
        regTxHash: string;
    }

    /**
     * @typedef {Object} TransitionPayloadJSON
     * @property {Number} version
     * @property {string} regTxId
     * @property {string} hashPrevSubTx
     * @property {Number} creditFee
     * @property {string} hashSTPacket
     * @property {string} [payloadSig]
     * @property {string} [payloadSigSize]
     */
    export type TransitionPayloadJSON = {
        version: number;
        regTxId: string;
        hashPrevSubTx: string;
        creditFee: number;
        hashSTPacket: string;
        payloadSig?: string;
        payloadSigSize?: string;
    };

    /**
     * @class SubTxTransitionPayload
     * @property {number} version
     * @property {string} regTxId
     * @property {string} hashPrevSubTx
     * @property {number} creditFee
     * @property {string} hashSTPacket
     * @property {string} [payloadSig]
     * @property {string} [payloadSigSize]
     */
    export class SubTxTransitionPayload {
        /**
         * Serialize transition payload
         * @param {TransitionPayloadJSON} transitionPayload
         * @return {Buffer} serialized payload
         */
        static serializeJSONToBuffer(transitionPayload: TransitionPayloadJSON): Buffer;

        /**
         * Parse raw transition payload
         * @param {Buffer} rawPayload
         * @return {SubTxTransitionPayload}
         */
        static fromBuffer(rawPayload: Buffer): SubTxTransitionPayload;

        /**
         * Create new instance of payload from JSON
         * @param {string|TransitionPayloadJSON} payloadJson
         * @return {SubTxTransitionPayload}
         */
        static fromJSON(payloadJson: string | TransitionPayloadJSON): SubTxTransitionPayload;

        /**
         * Validate payload
         * @param {TransitionPayloadJSON} blockchainUserPayload
         * @return {boolean}
         */
        static validatePayloadJSON(blockchainUserPayload: TransitionPayloadJSON): boolean;

        /**
         * Validates payload data
         * @return {boolean}
         */
        validate(): boolean;

        /**
         * @param {string} regTxId - Hex string
         */
        setRegTxId(regTxId: string): void;

        /**
         * @param {string} hashPrevSubTx - Hex string
         * @return {SubTxTransitionPayload}
         */
        setHashPrevSubTx(hashPrevSubTx: string): SubTxTransitionPayload;

        /**
         * @param {string} hashSTPacket - Hex string
         * @return {SubTxTransitionPayload}
         */
        setHashSTPacket(hashSTPacket: string): SubTxTransitionPayload;

        /**
         * @param {number} creditFee
         * @return {SubTxTransitionPayload}
         */
        setCreditFee(creditFee: number): SubTxTransitionPayload;

        /**
         * Serializes payload to JSON
         * @param [options]
         * @param {boolean} options.skipSignature - skip signature part. Needed for creating new signature
         * @return {TransitionPayloadJSON}
         */
        toJSON(options?: {
            skipSignature: boolean;
        }): TransitionPayloadJSON;

        /**
         * Serialize payload to buffer
         * @param [options]
         * @param {boolean} options.skipSignature - skip signature part. Needed for creating new signature
         * @return {Buffer}
         */
        toBuffer(options?: {
            skipSignature: boolean;
        }): Buffer;

        /**
         * Copy payload instance
         * @return {SubTxTransitionPayload}
         */
        copy(): SubTxTransitionPayload;

        version: number;
        regTxId: string;
        hashPrevSubTx: string;
        creditFee: number;
        hashSTPacket: string;
        payloadSig?: string;
        payloadSigSize?: string;
    }

    /**
     * @namespace Signing
     */
    export namespace Signing {
        /**
         * @function
         * Returns a buffer of length 32 bytes with the hash that needs to be signed
         * for OP_CHECKSIG.
         *
         * @name Signing.sighash
         * @param {Transaction} transaction the transaction to sign
         * @param {number} sighashType the type of the hash
         * @param {number} inputNumber the input index for the signature
         * @param {Script} subscript the script that will be signed
         */
        function sighash(transaction: Transaction, sighashType: number, inputNumber: number, subscript: Script): void;

        /**
         * Create a signature
         *
         * @function
         * @name Signing.sign
         * @param {Transaction} transaction
         * @param {PrivateKey} privateKey
         * @param {number} sighash
         * @param {number} inputIndex
         * @param {Script} subscript
         * @return {Signature}
         */
        function sign(transaction: Transaction, privateKey: PrivateKey, sighash: number, inputIndex: number, subscript: Script): Signature;

        /**
         * Verify a signature
         *
         * @function
         * @name Signing.verify
         * @param {Transaction} transaction
         * @param {Signature} signature
         * @param {PublicKey} publicKey
         * @param {number} inputIndex
         * @param {Script} subscript
         * @return {boolean}
         */
        function verify(transaction: Transaction, signature: Signature, publicKey: PublicKey, inputIndex: number, subscript: Script): boolean;
    }

    /**
     * @desc
     * Wrapper around Signature with fields related to signing a transaction specifically
     *
     * @param {Object|string|TransactionSignature} arg
     * @constructor
     */
    export class TransactionSignature {
        constructor(arg: any | string | TransactionSignature);

        /**
         * Serializes a transaction to a plain JS object
         *
         * @function
         * @return {Object}
         */
        toObject(): any;

        /**
         * Builds a TransactionSignature from an object
         * @param {Object} object
         * @return {TransactionSignature}
         */
        static fromObject(object: any): TransactionSignature;
    }

    /**
     * Represents a transaction, a set of inputs and outputs to change ownership of tokens
     *
     * @param {*} serialized
     * @constructor
     */
    export class Transaction {
        constructor(serialized: any);

        /**
         * Create a 'shallow' copy of the transaction, by serializing and deserializing
         * it dropping any additional information that inputs and outputs may have hold
         *
         * @param {Transaction} transaction
         * @return {Transaction}
         */
        static shallowCopy(transaction: Transaction): Transaction;

        /**
         * Retrieve the little endian hash of the transaction (used for serialization)
         * @return {Buffer}
         */
        _getHash(): Buffer;

        /**
         * Retrieve a hex string that can be used with bitcoind's CLI interface
         * (decoderawtransaction, sendrawtransaction)
         *
         * @param {Object|boolean=} unsafe if true, skip all tests. if it's an object,
         *   it's expected to contain a set of flags to skip certain tests:
         * * `disableAll`: disable all checks
         * * `disableSmallFees`: disable checking for fees that are too small
         * * `disableLargeFees`: disable checking for fees that are too large
         * * `disableIsFullySigned`: disable checking if all inputs are fully signed
         * * `disableDustOutputs`: disable checking if there are no outputs that are dust amounts
         * * `disableMoreOutputThanInput`: disable checking if the transaction spends more bitcoins than the sum of the input amounts
         * @return {string}
         */
        serialize(unsafe: any | boolean): string;

        /**
         * Retrieve a hex string that can be used with bitcoind's CLI interface
         * (decoderawtransaction, sendrawtransaction)
         *
         * @param {Object} opts allows to skip certain tests. {@see Transaction#serialize}
         * @return {string}
         */
        checkedSerialize(opts: any): string;

        /**
         * Retrieve a possible error that could appear when trying to serialize and
         * broadcast this transaction.
         *
         * @param {Object} opts allows to skip certain tests. {@see Transaction#serialize}
         * @return {bitcore.Error}
         */
        getSerializationError(opts: any): bitcore.Error;

        /**
         * Instant send fee is based on the number of inputs, not on the transaction size
         * @return {number}
         */
        estimateInstantSendFee(): number;

        /**
         * Sets nLockTime so that transaction is not valid until the desired date(a
         * timestamp in seconds since UNIX epoch is also accepted)
         *
         * @param {Date | Number} time
         * @return {Transaction} this
         */
        lockUntilDate(time: Date | number): Transaction;

        /**
         * Sets nLockTime so that transaction is not valid until the desired block
         * height.
         *
         * @param {Number} height
         * @return {Transaction} this
         */
        lockUntilBlockHeight(height: number): Transaction;

        /**
         *  Returns a semantic version of the transaction's nLockTime.
         *  @return {Number|Date}
         *  If nLockTime is 0, it returns null,
         *  if it is < 500000000, it returns a block height (number)
         *  else it returns a Date object.
         */
        getLockTime(): number | Date;

        /**
         * Add an input to this transaction. This is a high level interface
         * to add an input, for more control, use @{link Transaction#addInput}.
         *
         * Can receive, as output information, the output of bitcoind's `listunspent` command,
         * and a slightly fancier format recognized by bitcore:
         *
         * ```
         * {
         *  address: 'mszYqVnqKoQx4jcTdJXxwKAissE3Jbrrc1',
         *  txId: 'a477af6b2667c29670467e4e0728b685ee07b240235771862318e29ddbe58458',
         *  outputIndex: 0,
         *  script: Script.empty(),
         *  satoshis: 1020000
         * }
         * ```
         * Where `address` can be either a string or a bitcore Address object. The
         * same is true for `script`, which can be a string or a bitcore Script.
         *
         * Beware that this resets all the signatures for inputs (in further versions,
         * SIGHASH_SINGLE or SIGHASH_NONE signatures will not be reset).
         *
         * @example
         * ```javascript
         * var transaction = new Transaction();
         *
         * // From a pay to public key hash output from bitcoind's listunspent
         * transaction.from({'txid': '0000...', vout: 0, amount: 0.1, scriptPubKey: 'OP_DUP ...'});
         *
         * // From a pay to public key hash output
         * transaction.from({'txId': '0000...', outputIndex: 0, satoshis: 1000, script: 'OP_DUP ...'});
         *
         * // From a multisig P2SH output
         * transaction.from({'txId': '0000...', inputIndex: 0, satoshis: 1000, script: '... OP_HASH'},
         *                  ['03000...', '02000...'], 2);
         * ```
         *
         * @param {(Array<Transaction.fromObject>|Transaction.fromObject)} utxo
         * @param {Array=} pubkeys
         * @param {number=} threshold
         */
        from(utxo: Transaction.fromObject[] | Transaction.fromObject, pubkeys?: any[], threshold?: number): void;

        /**
         * Add an input to this transaction. The input must be an instance of the `Input` class.
         * It should have information about the Output that it's spending, but if it's not already
         * set, two additional parameters, `outputScript` and `satoshis` can be provided.
         *
         * @param {Input} input
         * @param {String|Script} outputScript
         * @param {number} satoshis
         * @return Transaction {this}, for chaining
         */
        addInput(input: Input, outputScript: string | Script, satoshis: number): this;

        /**
         * Add an input to this transaction, without checking that the input has information about
         * the output that it's spending.
         *
         * @param {Input} input
         * @return Transaction {this}, for chaining
         */
        uncheckedAddInput(input: Input): this;

        /**
         * Returns true if the transaction has enough info on all inputs to be correctly validated
         *
         * @return {boolean}
         */
        hasAllUtxoInfo(): boolean;

        /**
         * Manually set the fee for this transaction. Beware that this resets all the signatures
         * for inputs (in further versions, SIGHASH_SINGLE or SIGHASH_NONE signatures will not
         * be reset).
         *
         * @param {number} amount satoshis to be sent
         * @return {Transaction} this, for chaining
         */
        fee(amount: number): Transaction;

        /**
         * Manually set the fee per KB for this transaction. Beware that this resets all the signatures
         * for inputs (in further versions, SIGHASH_SINGLE or SIGHASH_NONE signatures will not
         * be reset).
         *
         * @param {number} amount satoshis per KB to be sent
         * @return {Transaction} this, for chaining
         */
        feePerKb(amount: number): Transaction;

        /**
         * Set the change address for this transaction
         *
         * Beware that this resets all the signatures for inputs (in further versions,
         * SIGHASH_SINGLE or SIGHASH_NONE signatures will not be reset).
         *
         * @param {Address} address An address for change to be sent to.
         * @return {Transaction} this, for chaining
         */
        change(address: Address): Transaction;

        /**
         * @return {Output} change output, if it exists
         */
        getChangeOutput(): Output;

        /**
         * Add an output to the transaction.
         *
         * Beware that this resets all the signatures for inputs (in further versions,
         * SIGHASH_SINGLE or SIGHASH_NONE signatures will not be reset).
         *
         * @param {(string|Address|Array.<Transaction.toObject>)} address
         * @param {number} amount in satoshis
         * @return {Transaction} this, for chaining
         */
        to(address: string | Address | Transaction.toObject[], amount: number): Transaction;

        /**
         * Add an OP_RETURN output to the transaction.
         *
         * Beware that this resets all the signatures for inputs (in further versions,
         * SIGHASH_SINGLE or SIGHASH_NONE signatures will not be reset).
         *
         * @param {Buffer|string} value the data to be stored in the OP_RETURN output.
         *    In case of a string, the UTF-8 representation will be stored
         * @return {Transaction} this, for chaining
         */
        addData(value: Buffer | string): Transaction;

        /**
         * Add an output to the transaction.
         *
         * @param {Output} output the output to add.
         * @return {Transaction} this, for chaining
         */
        addOutput(output: Output): Transaction;

        /**
         * Remove all outputs from the transaction.
         *
         * @return {Transaction} this, for chaining
         */
        clearOutputs(): Transaction;

        /**
         * Calculates or gets the total output amount in satoshis
         *
         * @return {Number} the transaction total output amount
         */
        _getOutputAmount(): number;

        /**
         * Calculates or gets the total input amount in satoshis
         *
         * @return {Number} the transaction total input amount
         */
        _getInputAmount(): number;

        /**
         * Calculates the fee of the transaction.
         *
         * If there's a fixed fee set, return that.
         *
         * If there is no change output set, the fee is the
         * total value of the outputs minus inputs. Note that
         * a serialized transaction only specifies the value
         * of its outputs. (The value of inputs are recorded
         * in the previous transaction outputs being spent.)
         * This method therefore raises a "MissingPreviousOutput"
         * error when called on a serialized transaction.
         *
         * If there's no fee set and no change address,
         * estimate the fee based on size.
         *
         * @return {Number} fee of this transaction in satoshis
         */
        getFee(): number;

        /**
         * Estimates fee from serialized transaction size in bytes.
         */
        _estimateFee(): void;

        /**
         * Sort a transaction's inputs and outputs according to BIP69
         *
         * @see {@link https://github.com/bitcoin/bips/blob/master/bip-0069.mediawiki}
         * @return {Transaction} this
         */
        sort(): Transaction;

        /**
         * Randomize this transaction's outputs ordering. The shuffling algorithm is a
         * version of the Fisher-Yates shuffle, provided by lodash's _.shuffle().
         *
         * @return {Transaction} this
         */
        shuffleOutputs(): Transaction;

        /**
         * Sort this transaction's outputs, according to a given sorting function that
         * takes an array as argument and returns a new array, with the same elements
         * but with a different order. The argument function MUST NOT modify the order
         * of the original array
         *
         * @param {Function} sortingFunction
         * @return {Transaction} this
         */
        sortOutputs(sortingFunction: (...params: any[]) => any): Transaction;

        /**
         * Sort this transaction's inputs, according to a given sorting function that
         * takes an array as argument and returns a new array, with the same elements
         * but with a different order.
         *
         * @param {Function} sortingFunction
         * @return {Transaction} this
         */
        sortInputs(sortingFunction: (...params: any[]) => any): Transaction;

        /**
         * Sign the transaction using one or more private keys.
         *
         * It tries to sign each input, verifying that the signature will be valid
         * (matches a public key).
         *
         * @param {Array|String|PrivateKey} privateKey
         * @param {number} [sigtype]
         * @return {Transaction} this, for chaining
         */
        sign(privateKey: any[] | string | PrivateKey, sigtype?: number): Transaction;

        /**
         * Add a signature to the transaction
         *
         * @param {Object} signature
         * @param {number} signature.inputIndex
         * @param {number} signature.sigtype
         * @param {PublicKey} signature.publicKey
         * @param {Signature} signature.signature
         * @return {Transaction} this, for chaining
         */
        applySignature(signature: {
            inputIndex: number;
            sigtype: number;
            publicKey: PublicKey;
            signature: Signature;
        }): Transaction;

        /**
         * @returns {bool} whether the signature is valid for this transaction input
         */
        verifySignature(): boolean;

        /**
         * Check that a transaction passes basic sanity tests. If not, return a string
         * describing the error. This function contains the same logic as
         * CheckTransaction in bitcoin core.
         */
        verify(): void;

        /**
         * Analogous to bitcoind's IsCoinBase function in transaction.h
         */
        isCoinbase(): void;

        /**
         * Determines if this transaction can be replaced in the mempool with another
         * transaction that provides a sufficiently higher fee (RBF).
         */
        isRBF(): void;

        /**
         * Enable this transaction to be replaced in the mempool (RBF) if a transaction
         * includes a sufficiently higher fee. It will set the sequenceNumber to
         * DEFAULT_RBF_SEQNUMBER for all inputs if the sequence number does not
         * already enable RBF.
         */
        enableRBF(): void;

        /**
         * Returns true if this transaction is qualified to be a simple transaction to the network (<= 4 inputs).
         * @returns {boolean}
         */
        isSimpleTransaction(): boolean;

        /**
         * Set special transaction type and create an empty extraPayload
         * @param {number} type
         * @returns {Transaction}
         */
        setType(type: number): Transaction;

        /**
         * Returns true if this transaction is DIP2 special transaction, returns false otherwise.
         * @returns {boolean}
         */
        isSpecialTransaction(): boolean;

        /**
         * Checks if transaction has DIP2 extra payload
         * @returns {boolean}
         */
        hasExtraPayload(): boolean;

        /**
         * @param {AbstractPayload} payload
         * @return {Transaction}
         */
        setExtraPayload(payload: AbstractPayload): Transaction;

        /**
         * Return extra payload size in bytes
         * @return {Number}
         */
        getExtraPayloadSize(): number;
    }

    export namespace Transaction {
        /**
         * @typedef {Object} Transaction.fromObject
         * @property {string} prevTxId
         * @property {number} outputIndex
         * @property {(Buffer|string|Script)} script
         * @property {number} satoshis
         */
        type fromObject = {
            prevTxId: string;
            outputIndex: number;
            script: Buffer | string | Script;
            satoshis: number;
        };
        /**
         * @typedef {Object} Transaction.toObject
         * @property {(string|Address)} address
         * @property {number} satoshis
         */
        type toObject = {
            address: string | Address;
            satoshis: number;
        };
    }

    /**
     * Represents an unspent output information: its script, associated amount and address,
     * transaction id and output index.
     *
     * @constructor
     * @param {object} data
     * @param {string} data.txid the previous transaction id
     * @param {string=} data.txId alias for `txid`
     * @param {number} data.vout the index in the transaction
     * @param {number=} data.outputIndex alias for `vout`
     * @param {string|Script} data.scriptPubKey the script that must be resolved to release the funds
     * @param {string|Script=} data.script alias for `scriptPubKey`
     * @param {number} data.amount amount of bitcoins associated
     * @param {number=} data.satoshis alias for `amount`, but expressed in satoshis (1 BTC = 1e8 satoshis)
     * @param {string|Address=} data.address the associated address to the script, if provided
     */
    export class UnspentOutput {
        constructor(data: {
            txid: string;
            txId?: string;
            vout: number;
            outputIndex?: number;
            scriptPubKey: string | Script;
            script: string | Script;
            amount: number;
            satoshis?: number;
            address: string | Address;
        });

        /**
         * Provide an informative output when displaying this object in the console
         *
         * @return {string}
         */
        inspect(): string;

        /**
         * String representation: just "txid:index"
         *
         * @return {string}
         */
        toString(): string;

        /**
         * Deserialize an UnspentOutput from an object
         *
         * @param {object|string} data
         * @returns {UnspentOutput}
         */
        static fromObject(data: any | string): UnspentOutput;

        /**
         * Returns a plain object (no prototype or methods) with the associated info for this output
         *
         * @function
         * @return {object}
         */
        toObject(): any;
    }

    /**
     * Utility for handling and converting bitcoins units. The supported units are
     * BTC, mBTC, bits (also named uBTC) and satoshis. A unit instance can be created with an
     * amount and a unit code, or alternatively using static methods like {fromBTC}.
     * It also allows to be created from a fiat amount and the exchange rate, or
     * alternatively using the {fromFiat} static method.
     * You can consult for different representation of a unit instance using it's
     * {to} method, the fixed unit methods like {toSatoshis} or alternatively using
     * the unit accessors. It also can be converted to a fiat amount by providing the
     * corresponding BTC/fiat exchange rate.
     *
     * @example
     * ```javascript
     * var sats = Unit.fromBTC(1.3).toSatoshis();
     * var mili = Unit.fromBits(1.3).to(Unit.mBTC);
     * var bits = Unit.fromFiat(1.3, 350).bits;
     * var btc = new Unit(1.3, Unit.bits).BTC;
     * ```
     *
     * @param {Number} amount - The amount to be represented
     * @param {String|Number} code - The unit of the amount or the exchange rate
     * @returns {Unit} A new instance of a Unit
     * @constructor
     */
    export class Unit {
        constructor(amount: number, code: string | number);

        /**
         * Returns a Unit instance created from JSON string or object
         *
         * @param {String|Object} json - JSON with keys: amount and code
         * @returns {Unit} A Unit instance
         */
        static fromObject(json: string | any): Unit;

        /**
         * Returns a Unit instance created from an amount in BTC
         *
         * @param {Number} amount - The amount in BTC
         * @returns {Unit} A Unit instance
         */
        static fromBTC(amount: number): Unit;

        /**
         * Returns a Unit instance created from an amount in mBTC
         *
         * @function
         * @param {Number} amount - The amount in mBTC
         * @returns {Unit} A Unit instance
         */
        static fromMillis(amount: number): Unit;

        /**
         * Returns a Unit instance created from an amount in bits
         *
         * @function
         * @param {Number} amount - The amount in bits
         * @returns {Unit} A Unit instance
         */
        static fromMicros(amount: number): Unit;

        /**
         * Returns a Unit instance created from an amount in satoshis
         *
         * @param {Number} amount - The amount in satoshis
         * @returns {Unit} A Unit instance
         */
        static fromSatoshis(amount: number): Unit;

        /**
         * Returns a Unit instance created from a fiat amount and exchange rate.
         *
         * @param {Number} amount - The amount in fiat
         * @param {Number} rate - The exchange rate BTC/fiat
         * @returns {Unit} A Unit instance
         */
        static fromFiat(amount: number, rate: number): Unit;

        /**
         * Returns the value represented in the specified unit
         *
         * @param {String|Number} code - The unit code or exchange rate
         * @returns {Number} The converted value
         */
        to(code: string | number): number;

        /**
         * Returns the value represented in BTC
         *
         * @returns {Number} The value converted to BTC
         */
        toBTC(): number;

        /**
         * Returns the value represented in mBTC
         *
         * @function
         * @returns {Number} The value converted to mBTC
         */
        toMillis(): number;

        /**
         * Returns the value represented in bits
         *
         * @function
         * @returns {Number} The value converted to bits
         */
        toMicros(): number;

        /**
         * Returns the value represented in satoshis
         *
         * @returns {Number} The value converted to satoshis
         */
        toSatoshis(): number;

        /**
         * Returns the value represented in fiat
         *
         * @param {string} rate - The exchange rate between BTC/currency
         * @returns {Number} The value converted to satoshis
         */
        atRate(rate: string): number;

        /**
         * Returns a string representation of the value in satoshis
         *
         * @returns {string} the value in satoshis
         */
        toString(): string;

        /**
         * Returns a plain object representation of the Unit
         *
         * @function
         * @returns {Object} An object with the keys: amount and code
         */
        toObject(): any;

        /**
         * Returns a string formatted for the console
         *
         * @returns {string} the value in satoshis
         */
        inspect(): string;
    }

    /**
     * Bitcore URI
     *
     * Instantiate an URI from a Bitcoin URI String or an Object. A URI instance
     * can be created with a Bitcoin URI string or an object. All instances of
     * URI are valid, the static method isValid allows checking before instantiation.
     *
     * All standard parameters can be found as members of the class, the address
     * is represented using an {Address} instance and the amount is represented in
     * satoshis. Any other non-standard parameters can be found under the extra member.
     *
     * @example
     * ```javascript
     *
     * var uri = new URI('dash:XsV4GHVKGTjQFvwB7c6mYsGV3Mxf7iser6?amount=1.2');
     * console.log(uri.address, uri.amount);
     * ```
     *
     * @param {string|Object} data - A Bitcoin URI string or an Object
     * @param {Array.<string>=} knownParams - Required non-standard params
     * @throws {TypeError} Invalid bitcoin address
     * @throws {TypeError} Invalid amount
     * @throws {Error} Unknown required argument
     * @returns {URI} A new valid and frozen instance of URI
     * @constructor
     */
    export class URI {
        constructor(data: string | any, knownParams?: string[]);

        /**
         * Instantiate a URI from a String
         *
         * @param {string} str - JSON string or object of the URI
         * @returns {URI} A new instance of a URI
         */
        static fromString(str: string): URI;

        /**
         * Instantiate a URI from an Object
         *
         * @param {Object} data - object of the URI
         * @returns {URI} A new instance of a URI
         */
        static fromObject(data: any): URI;

        /**
         * Check if an bitcoin URI string is valid
         *
         * @example
         * ```javascript
         *
         * var valid = URI.isValid('dash:XsV4GHVKGTjQFvwB7c6mYsGV3Mxf7iser6');
         * // true
         * ```
         *
         * @param {string|Object} data - A bitcoin URI string or an Object
         * @param {Array.<string>=} knownParams - Required non-standard params
         * @returns {boolean} Result of uri validation
         */
        static isValid(data: string | any, knownParams?: string[]): boolean;

        /**
         * Convert a bitcoin URI string into a simple object.
         *
         * @param {string} uri - A bitcoin URI string
         * @throws {TypeError} Invalid bitcoin URI
         * @returns {Object} An object with the parsed params
         */
        static parse(uri: string): any;

        /**
         * Internal function to load the URI instance with an object.
         *
         * @param {Object} obj - Object with the information
         * @throws {TypeError} Invalid bitcoin address
         * @throws {TypeError} Invalid amount
         * @throws {Error} Unknown required argument
         */
        _fromObject(obj: any): void;

        /**
         * Internal function to transform a BTC string amount into satoshis
         *
         * @param {string} amount - Amount BTC string
         * @throws {TypeError} Invalid amount
         * @returns {Object} Amount represented in satoshis
         */
        _parseAmount(amount: string): any;

        /**
         * Will return a string representation of the URI
         *
         * @returns {string} Bitcoin URI string
         */
        toString(): string;

        /**
         * Will return a string formatted for the console
         *
         * @returns {string} Bitcoin URI
         */
        inspect(): string;
    }

    /**
     * @param {string} bitString
     * @return {boolean}
     */
    export function isBitString(bitString: string): boolean;

    /**
     * Converts boolean array to uint8 array, i.e:
     * [true, true, true, true, true, true, true, true] will be converted to [255]
     * @param {boolean[]|number[]} bitArray
     * @param {boolean} [reverseBits]
     * @return {number[]}
     */
    export function convertBitArrayToUInt8Array(bitArray: boolean[] | number[], reverseBits?: boolean): number[];

    /**
     * Converts a bit string, i.e. '1000101010101010100' to an array with 8 bit unsigned integers
     * @param {string} bitString
     * @param {boolean} reverseBits
     * @return {number[]}
     */
    export function bitStringToUInt8Array(bitString: string, reverseBits: boolean): number[];

    /**
     * Maps ipv4:port to ipv6 buffer and port
     * Note: this is made mostly for the deterministic masternode list, which are ipv4 addresses encoded as ipv6 addresses
     * @param {string} string
     * @return {Buffer}
     */
    export function ipAndPortToBuffer(string: string): Buffer;

    /**
     * Parses ipv6 buffer and port to ipv4:port string
     * @param {Buffer} buffer
     * @return {string}
     */
    export function bufferToIPAndPort(buffer: Buffer): string;

    /**
     * Checks if string is an ipv4 address
     * @param {string} ipAndPortString
     * @return {boolean}
     */
    export function isIpV4(ipAndPortString: string): boolean;

    /**
     * @param {string} address
     * @return {boolean}
     */
    export function isZeroAddress(address: string): boolean;

    /**
     * @namespace JSUtil
     */
    export namespace JSUtil {
        /**
         * Determines whether a string contains only hexadecimal values
         *
         * @function
         * @name JSUtil.isHexa
         * @param {string} value
         * @return {boolean} true if the string is the hex representation of a number
         */
        function isHexa(value: string): boolean;
    }

    /**
     * Builds a merkle tree of all passed hashes
     * @link https://en.bitcoin.it/wiki/Protocol_specification#Merkle_Trees
     * @param {Buffer[]} hashes
     * @returns {Buffer[]} - An array with each level of the tree after the other.
     */
    export function getMerkleTree(hashes: Buffer[]): Buffer[];

    /**
     * Copies root of the passed tree to a new Buffer and returns it
     * @param {Buffer[]} merkleTree
     * @returns {Buffer|undefined} - A buffer of the merkle root hash
     */
    export function getMerkleRoot(merkleTree: Buffer[]): Buffer | undefined;

    /**
     * Helper function to efficiently calculate the number of nodes at given height in the merkle tree
     * @param {number} totalElementsCount
     * @param {number} height
     * @return {number}
     */
    export function calculateTreeWidth(totalElementsCount: number, height: number): number;

    /**
     * @param {number} hashesCount
     * @return {number}
     */
    export function calculateTreeHeight(hashesCount: number): number;

    /**
     *
     * @param {number} height
     * @param {number} position
     * @param {Buffer[]} hashes
     * @return {Buffer}
     */
    export function calculateHashAtHeight(height: number, position: number, hashes: Buffer[]): Buffer;

    /**
     * @param {number} height
     * @param {number} position
     * @param {Buffer[]} hashes
     * @param {boolean[]} matches
     * @return {{flags: boolean[], merkleHashes: string[]}}
     */
    export function traverseAndBuildPartialTree(height: number, position: number, hashes: Buffer[], matches: boolean[]): any;

}
