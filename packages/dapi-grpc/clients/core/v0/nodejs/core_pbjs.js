/*eslint-disable block-scoped-var, id-length, no-control-regex, no-magic-numbers, no-prototype-builtins, no-redeclare, no-shadow, no-var, sort-vars*/
"use strict";

var $protobuf = require("protobufjs/minimal");

// Common aliases
var $Reader = $protobuf.Reader, $Writer = $protobuf.Writer, $util = $protobuf.util;

// Exported root namespace
var $root = $protobuf.roots.core_root || ($protobuf.roots.core_root = {});

$root.org = (function() {

    /**
     * Namespace org.
     * @exports org
     * @namespace
     */
    var org = {};

    org.dash = (function() {

        /**
         * Namespace dash.
         * @memberof org
         * @namespace
         */
        var dash = {};

        dash.platform = (function() {

            /**
             * Namespace platform.
             * @memberof org.dash
             * @namespace
             */
            var platform = {};

            platform.dapi = (function() {

                /**
                 * Namespace dapi.
                 * @memberof org.dash.platform
                 * @namespace
                 */
                var dapi = {};

                dapi.v0 = (function() {

                    /**
                     * Namespace v0.
                     * @memberof org.dash.platform.dapi
                     * @namespace
                     */
                    var v0 = {};

                    v0.Core = (function() {

                        /**
                         * Constructs a new Core service.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a Core
                         * @extends $protobuf.rpc.Service
                         * @constructor
                         * @param {$protobuf.RPCImpl} rpcImpl RPC implementation
                         * @param {boolean} [requestDelimited=false] Whether requests are length-delimited
                         * @param {boolean} [responseDelimited=false] Whether responses are length-delimited
                         */
                        function Core(rpcImpl, requestDelimited, responseDelimited) {
                            $protobuf.rpc.Service.call(this, rpcImpl, requestDelimited, responseDelimited);
                        }

                        (Core.prototype = Object.create($protobuf.rpc.Service.prototype)).constructor = Core;

                        /**
                         * Creates new Core service using the specified rpc implementation.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.Core
                         * @static
                         * @param {$protobuf.RPCImpl} rpcImpl RPC implementation
                         * @param {boolean} [requestDelimited=false] Whether requests are length-delimited
                         * @param {boolean} [responseDelimited=false] Whether responses are length-delimited
                         * @returns {Core} RPC service. Useful where requests and/or responses are streamed.
                         */
                        Core.create = function create(rpcImpl, requestDelimited, responseDelimited) {
                            return new this(rpcImpl, requestDelimited, responseDelimited);
                        };

                        /**
                         * Callback as used by {@link org.dash.platform.dapi.v0.Core#getStatus}.
                         * @memberof org.dash.platform.dapi.v0.Core
                         * @typedef getStatusCallback
                         * @type {function}
                         * @param {Error|null} error Error, if any
                         * @param {org.dash.platform.dapi.v0.GetStatusResponse} [response] GetStatusResponse
                         */

                        /**
                         * Calls getStatus.
                         * @function getStatus
                         * @memberof org.dash.platform.dapi.v0.Core
                         * @instance
                         * @param {org.dash.platform.dapi.v0.IGetStatusRequest} request GetStatusRequest message or plain object
                         * @param {org.dash.platform.dapi.v0.Core.getStatusCallback} callback Node-style callback called with the error, if any, and GetStatusResponse
                         * @returns {undefined}
                         * @variation 1
                         */
                        Object.defineProperty(Core.prototype.getStatus = function getStatus(request, callback) {
                            return this.rpcCall(getStatus, $root.org.dash.platform.dapi.v0.GetStatusRequest, $root.org.dash.platform.dapi.v0.GetStatusResponse, request, callback);
                        }, "name", { value: "getStatus" });

                        /**
                         * Calls getStatus.
                         * @function getStatus
                         * @memberof org.dash.platform.dapi.v0.Core
                         * @instance
                         * @param {org.dash.platform.dapi.v0.IGetStatusRequest} request GetStatusRequest message or plain object
                         * @returns {Promise<org.dash.platform.dapi.v0.GetStatusResponse>} Promise
                         * @variation 2
                         */

                        /**
                         * Callback as used by {@link org.dash.platform.dapi.v0.Core#getBlock}.
                         * @memberof org.dash.platform.dapi.v0.Core
                         * @typedef getBlockCallback
                         * @type {function}
                         * @param {Error|null} error Error, if any
                         * @param {org.dash.platform.dapi.v0.GetBlockResponse} [response] GetBlockResponse
                         */

                        /**
                         * Calls getBlock.
                         * @function getBlock
                         * @memberof org.dash.platform.dapi.v0.Core
                         * @instance
                         * @param {org.dash.platform.dapi.v0.IGetBlockRequest} request GetBlockRequest message or plain object
                         * @param {org.dash.platform.dapi.v0.Core.getBlockCallback} callback Node-style callback called with the error, if any, and GetBlockResponse
                         * @returns {undefined}
                         * @variation 1
                         */
                        Object.defineProperty(Core.prototype.getBlock = function getBlock(request, callback) {
                            return this.rpcCall(getBlock, $root.org.dash.platform.dapi.v0.GetBlockRequest, $root.org.dash.platform.dapi.v0.GetBlockResponse, request, callback);
                        }, "name", { value: "getBlock" });

                        /**
                         * Calls getBlock.
                         * @function getBlock
                         * @memberof org.dash.platform.dapi.v0.Core
                         * @instance
                         * @param {org.dash.platform.dapi.v0.IGetBlockRequest} request GetBlockRequest message or plain object
                         * @returns {Promise<org.dash.platform.dapi.v0.GetBlockResponse>} Promise
                         * @variation 2
                         */

                        /**
                         * Callback as used by {@link org.dash.platform.dapi.v0.Core#broadcastTransaction}.
                         * @memberof org.dash.platform.dapi.v0.Core
                         * @typedef broadcastTransactionCallback
                         * @type {function}
                         * @param {Error|null} error Error, if any
                         * @param {org.dash.platform.dapi.v0.BroadcastTransactionResponse} [response] BroadcastTransactionResponse
                         */

                        /**
                         * Calls broadcastTransaction.
                         * @function broadcastTransaction
                         * @memberof org.dash.platform.dapi.v0.Core
                         * @instance
                         * @param {org.dash.platform.dapi.v0.IBroadcastTransactionRequest} request BroadcastTransactionRequest message or plain object
                         * @param {org.dash.platform.dapi.v0.Core.broadcastTransactionCallback} callback Node-style callback called with the error, if any, and BroadcastTransactionResponse
                         * @returns {undefined}
                         * @variation 1
                         */
                        Object.defineProperty(Core.prototype.broadcastTransaction = function broadcastTransaction(request, callback) {
                            return this.rpcCall(broadcastTransaction, $root.org.dash.platform.dapi.v0.BroadcastTransactionRequest, $root.org.dash.platform.dapi.v0.BroadcastTransactionResponse, request, callback);
                        }, "name", { value: "broadcastTransaction" });

                        /**
                         * Calls broadcastTransaction.
                         * @function broadcastTransaction
                         * @memberof org.dash.platform.dapi.v0.Core
                         * @instance
                         * @param {org.dash.platform.dapi.v0.IBroadcastTransactionRequest} request BroadcastTransactionRequest message or plain object
                         * @returns {Promise<org.dash.platform.dapi.v0.BroadcastTransactionResponse>} Promise
                         * @variation 2
                         */

                        /**
                         * Callback as used by {@link org.dash.platform.dapi.v0.Core#getTransaction}.
                         * @memberof org.dash.platform.dapi.v0.Core
                         * @typedef getTransactionCallback
                         * @type {function}
                         * @param {Error|null} error Error, if any
                         * @param {org.dash.platform.dapi.v0.GetTransactionResponse} [response] GetTransactionResponse
                         */

                        /**
                         * Calls getTransaction.
                         * @function getTransaction
                         * @memberof org.dash.platform.dapi.v0.Core
                         * @instance
                         * @param {org.dash.platform.dapi.v0.IGetTransactionRequest} request GetTransactionRequest message or plain object
                         * @param {org.dash.platform.dapi.v0.Core.getTransactionCallback} callback Node-style callback called with the error, if any, and GetTransactionResponse
                         * @returns {undefined}
                         * @variation 1
                         */
                        Object.defineProperty(Core.prototype.getTransaction = function getTransaction(request, callback) {
                            return this.rpcCall(getTransaction, $root.org.dash.platform.dapi.v0.GetTransactionRequest, $root.org.dash.platform.dapi.v0.GetTransactionResponse, request, callback);
                        }, "name", { value: "getTransaction" });

                        /**
                         * Calls getTransaction.
                         * @function getTransaction
                         * @memberof org.dash.platform.dapi.v0.Core
                         * @instance
                         * @param {org.dash.platform.dapi.v0.IGetTransactionRequest} request GetTransactionRequest message or plain object
                         * @returns {Promise<org.dash.platform.dapi.v0.GetTransactionResponse>} Promise
                         * @variation 2
                         */

                        /**
                         * Callback as used by {@link org.dash.platform.dapi.v0.Core#getEstimatedTransactionFee}.
                         * @memberof org.dash.platform.dapi.v0.Core
                         * @typedef getEstimatedTransactionFeeCallback
                         * @type {function}
                         * @param {Error|null} error Error, if any
                         * @param {org.dash.platform.dapi.v0.GetEstimatedTransactionFeeResponse} [response] GetEstimatedTransactionFeeResponse
                         */

                        /**
                         * Calls getEstimatedTransactionFee.
                         * @function getEstimatedTransactionFee
                         * @memberof org.dash.platform.dapi.v0.Core
                         * @instance
                         * @param {org.dash.platform.dapi.v0.IGetEstimatedTransactionFeeRequest} request GetEstimatedTransactionFeeRequest message or plain object
                         * @param {org.dash.platform.dapi.v0.Core.getEstimatedTransactionFeeCallback} callback Node-style callback called with the error, if any, and GetEstimatedTransactionFeeResponse
                         * @returns {undefined}
                         * @variation 1
                         */
                        Object.defineProperty(Core.prototype.getEstimatedTransactionFee = function getEstimatedTransactionFee(request, callback) {
                            return this.rpcCall(getEstimatedTransactionFee, $root.org.dash.platform.dapi.v0.GetEstimatedTransactionFeeRequest, $root.org.dash.platform.dapi.v0.GetEstimatedTransactionFeeResponse, request, callback);
                        }, "name", { value: "getEstimatedTransactionFee" });

                        /**
                         * Calls getEstimatedTransactionFee.
                         * @function getEstimatedTransactionFee
                         * @memberof org.dash.platform.dapi.v0.Core
                         * @instance
                         * @param {org.dash.platform.dapi.v0.IGetEstimatedTransactionFeeRequest} request GetEstimatedTransactionFeeRequest message or plain object
                         * @returns {Promise<org.dash.platform.dapi.v0.GetEstimatedTransactionFeeResponse>} Promise
                         * @variation 2
                         */

                        /**
                         * Callback as used by {@link org.dash.platform.dapi.v0.Core#subscribeToBlockHeadersWithChainLocks}.
                         * @memberof org.dash.platform.dapi.v0.Core
                         * @typedef subscribeToBlockHeadersWithChainLocksCallback
                         * @type {function}
                         * @param {Error|null} error Error, if any
                         * @param {org.dash.platform.dapi.v0.BlockHeadersWithChainLocksResponse} [response] BlockHeadersWithChainLocksResponse
                         */

                        /**
                         * Calls subscribeToBlockHeadersWithChainLocks.
                         * @function subscribeToBlockHeadersWithChainLocks
                         * @memberof org.dash.platform.dapi.v0.Core
                         * @instance
                         * @param {org.dash.platform.dapi.v0.IBlockHeadersWithChainLocksRequest} request BlockHeadersWithChainLocksRequest message or plain object
                         * @param {org.dash.platform.dapi.v0.Core.subscribeToBlockHeadersWithChainLocksCallback} callback Node-style callback called with the error, if any, and BlockHeadersWithChainLocksResponse
                         * @returns {undefined}
                         * @variation 1
                         */
                        Object.defineProperty(Core.prototype.subscribeToBlockHeadersWithChainLocks = function subscribeToBlockHeadersWithChainLocks(request, callback) {
                            return this.rpcCall(subscribeToBlockHeadersWithChainLocks, $root.org.dash.platform.dapi.v0.BlockHeadersWithChainLocksRequest, $root.org.dash.platform.dapi.v0.BlockHeadersWithChainLocksResponse, request, callback);
                        }, "name", { value: "subscribeToBlockHeadersWithChainLocks" });

                        /**
                         * Calls subscribeToBlockHeadersWithChainLocks.
                         * @function subscribeToBlockHeadersWithChainLocks
                         * @memberof org.dash.platform.dapi.v0.Core
                         * @instance
                         * @param {org.dash.platform.dapi.v0.IBlockHeadersWithChainLocksRequest} request BlockHeadersWithChainLocksRequest message or plain object
                         * @returns {Promise<org.dash.platform.dapi.v0.BlockHeadersWithChainLocksResponse>} Promise
                         * @variation 2
                         */

                        /**
                         * Callback as used by {@link org.dash.platform.dapi.v0.Core#subscribeToTransactionsWithProofs}.
                         * @memberof org.dash.platform.dapi.v0.Core
                         * @typedef subscribeToTransactionsWithProofsCallback
                         * @type {function}
                         * @param {Error|null} error Error, if any
                         * @param {org.dash.platform.dapi.v0.TransactionsWithProofsResponse} [response] TransactionsWithProofsResponse
                         */

                        /**
                         * Calls subscribeToTransactionsWithProofs.
                         * @function subscribeToTransactionsWithProofs
                         * @memberof org.dash.platform.dapi.v0.Core
                         * @instance
                         * @param {org.dash.platform.dapi.v0.ITransactionsWithProofsRequest} request TransactionsWithProofsRequest message or plain object
                         * @param {org.dash.platform.dapi.v0.Core.subscribeToTransactionsWithProofsCallback} callback Node-style callback called with the error, if any, and TransactionsWithProofsResponse
                         * @returns {undefined}
                         * @variation 1
                         */
                        Object.defineProperty(Core.prototype.subscribeToTransactionsWithProofs = function subscribeToTransactionsWithProofs(request, callback) {
                            return this.rpcCall(subscribeToTransactionsWithProofs, $root.org.dash.platform.dapi.v0.TransactionsWithProofsRequest, $root.org.dash.platform.dapi.v0.TransactionsWithProofsResponse, request, callback);
                        }, "name", { value: "subscribeToTransactionsWithProofs" });

                        /**
                         * Calls subscribeToTransactionsWithProofs.
                         * @function subscribeToTransactionsWithProofs
                         * @memberof org.dash.platform.dapi.v0.Core
                         * @instance
                         * @param {org.dash.platform.dapi.v0.ITransactionsWithProofsRequest} request TransactionsWithProofsRequest message or plain object
                         * @returns {Promise<org.dash.platform.dapi.v0.TransactionsWithProofsResponse>} Promise
                         * @variation 2
                         */

                        return Core;
                    })();

                    v0.GetStatusRequest = (function() {

                        /**
                         * Properties of a GetStatusRequest.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface IGetStatusRequest
                         */

                        /**
                         * Constructs a new GetStatusRequest.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a GetStatusRequest.
                         * @implements IGetStatusRequest
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.IGetStatusRequest=} [properties] Properties to set
                         */
                        function GetStatusRequest(properties) {
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * Creates a new GetStatusRequest instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.GetStatusRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetStatusRequest=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.GetStatusRequest} GetStatusRequest instance
                         */
                        GetStatusRequest.create = function create(properties) {
                            return new GetStatusRequest(properties);
                        };

                        /**
                         * Encodes the specified GetStatusRequest message. Does not implicitly {@link org.dash.platform.dapi.v0.GetStatusRequest.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.GetStatusRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetStatusRequest} message GetStatusRequest message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        GetStatusRequest.encode = function encode(message, writer) {
                            if (!writer)
                                writer = $Writer.create();
                            return writer;
                        };

                        /**
                         * Encodes the specified GetStatusRequest message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.GetStatusRequest.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.GetStatusRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetStatusRequest} message GetStatusRequest message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        GetStatusRequest.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes a GetStatusRequest message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.GetStatusRequest
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.GetStatusRequest} GetStatusRequest
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        GetStatusRequest.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.GetStatusRequest();
                            while (reader.pos < end) {
                                var tag = reader.uint32();
                                switch (tag >>> 3) {
                                default:
                                    reader.skipType(tag & 7);
                                    break;
                                }
                            }
                            return message;
                        };

                        /**
                         * Decodes a GetStatusRequest message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.GetStatusRequest
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.GetStatusRequest} GetStatusRequest
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        GetStatusRequest.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies a GetStatusRequest message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.GetStatusRequest
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        GetStatusRequest.verify = function verify(message) {
                            if (typeof message !== "object" || message === null)
                                return "object expected";
                            return null;
                        };

                        /**
                         * Creates a GetStatusRequest message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.GetStatusRequest
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.GetStatusRequest} GetStatusRequest
                         */
                        GetStatusRequest.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.GetStatusRequest)
                                return object;
                            return new $root.org.dash.platform.dapi.v0.GetStatusRequest();
                        };

                        /**
                         * Creates a plain object from a GetStatusRequest message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.GetStatusRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.GetStatusRequest} message GetStatusRequest
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        GetStatusRequest.toObject = function toObject() {
                            return {};
                        };

                        /**
                         * Converts this GetStatusRequest to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.GetStatusRequest
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        GetStatusRequest.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        return GetStatusRequest;
                    })();

                    v0.GetStatusResponse = (function() {

                        /**
                         * Properties of a GetStatusResponse.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface IGetStatusResponse
                         * @property {org.dash.platform.dapi.v0.GetStatusResponse.IVersion|null} [version] GetStatusResponse version
                         * @property {org.dash.platform.dapi.v0.GetStatusResponse.ITime|null} [time] GetStatusResponse time
                         * @property {org.dash.platform.dapi.v0.GetStatusResponse.Status|null} [status] GetStatusResponse status
                         * @property {number|null} [syncProgress] GetStatusResponse syncProgress
                         * @property {org.dash.platform.dapi.v0.GetStatusResponse.IChain|null} [chain] GetStatusResponse chain
                         * @property {org.dash.platform.dapi.v0.GetStatusResponse.IMasternode|null} [masternode] GetStatusResponse masternode
                         * @property {org.dash.platform.dapi.v0.GetStatusResponse.INetwork|null} [network] GetStatusResponse network
                         */

                        /**
                         * Constructs a new GetStatusResponse.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a GetStatusResponse.
                         * @implements IGetStatusResponse
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.IGetStatusResponse=} [properties] Properties to set
                         */
                        function GetStatusResponse(properties) {
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * GetStatusResponse version.
                         * @member {org.dash.platform.dapi.v0.GetStatusResponse.IVersion|null|undefined} version
                         * @memberof org.dash.platform.dapi.v0.GetStatusResponse
                         * @instance
                         */
                        GetStatusResponse.prototype.version = null;

                        /**
                         * GetStatusResponse time.
                         * @member {org.dash.platform.dapi.v0.GetStatusResponse.ITime|null|undefined} time
                         * @memberof org.dash.platform.dapi.v0.GetStatusResponse
                         * @instance
                         */
                        GetStatusResponse.prototype.time = null;

                        /**
                         * GetStatusResponse status.
                         * @member {org.dash.platform.dapi.v0.GetStatusResponse.Status} status
                         * @memberof org.dash.platform.dapi.v0.GetStatusResponse
                         * @instance
                         */
                        GetStatusResponse.prototype.status = 0;

                        /**
                         * GetStatusResponse syncProgress.
                         * @member {number} syncProgress
                         * @memberof org.dash.platform.dapi.v0.GetStatusResponse
                         * @instance
                         */
                        GetStatusResponse.prototype.syncProgress = 0;

                        /**
                         * GetStatusResponse chain.
                         * @member {org.dash.platform.dapi.v0.GetStatusResponse.IChain|null|undefined} chain
                         * @memberof org.dash.platform.dapi.v0.GetStatusResponse
                         * @instance
                         */
                        GetStatusResponse.prototype.chain = null;

                        /**
                         * GetStatusResponse masternode.
                         * @member {org.dash.platform.dapi.v0.GetStatusResponse.IMasternode|null|undefined} masternode
                         * @memberof org.dash.platform.dapi.v0.GetStatusResponse
                         * @instance
                         */
                        GetStatusResponse.prototype.masternode = null;

                        /**
                         * GetStatusResponse network.
                         * @member {org.dash.platform.dapi.v0.GetStatusResponse.INetwork|null|undefined} network
                         * @memberof org.dash.platform.dapi.v0.GetStatusResponse
                         * @instance
                         */
                        GetStatusResponse.prototype.network = null;

                        /**
                         * Creates a new GetStatusResponse instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.GetStatusResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetStatusResponse=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.GetStatusResponse} GetStatusResponse instance
                         */
                        GetStatusResponse.create = function create(properties) {
                            return new GetStatusResponse(properties);
                        };

                        /**
                         * Encodes the specified GetStatusResponse message. Does not implicitly {@link org.dash.platform.dapi.v0.GetStatusResponse.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.GetStatusResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetStatusResponse} message GetStatusResponse message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        GetStatusResponse.encode = function encode(message, writer) {
                            if (!writer)
                                writer = $Writer.create();
                            if (message.version != null && Object.hasOwnProperty.call(message, "version"))
                                $root.org.dash.platform.dapi.v0.GetStatusResponse.Version.encode(message.version, writer.uint32(/* id 1, wireType 2 =*/10).fork()).ldelim();
                            if (message.time != null && Object.hasOwnProperty.call(message, "time"))
                                $root.org.dash.platform.dapi.v0.GetStatusResponse.Time.encode(message.time, writer.uint32(/* id 2, wireType 2 =*/18).fork()).ldelim();
                            if (message.status != null && Object.hasOwnProperty.call(message, "status"))
                                writer.uint32(/* id 3, wireType 0 =*/24).int32(message.status);
                            if (message.syncProgress != null && Object.hasOwnProperty.call(message, "syncProgress"))
                                writer.uint32(/* id 4, wireType 1 =*/33).double(message.syncProgress);
                            if (message.chain != null && Object.hasOwnProperty.call(message, "chain"))
                                $root.org.dash.platform.dapi.v0.GetStatusResponse.Chain.encode(message.chain, writer.uint32(/* id 5, wireType 2 =*/42).fork()).ldelim();
                            if (message.masternode != null && Object.hasOwnProperty.call(message, "masternode"))
                                $root.org.dash.platform.dapi.v0.GetStatusResponse.Masternode.encode(message.masternode, writer.uint32(/* id 6, wireType 2 =*/50).fork()).ldelim();
                            if (message.network != null && Object.hasOwnProperty.call(message, "network"))
                                $root.org.dash.platform.dapi.v0.GetStatusResponse.Network.encode(message.network, writer.uint32(/* id 7, wireType 2 =*/58).fork()).ldelim();
                            return writer;
                        };

                        /**
                         * Encodes the specified GetStatusResponse message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.GetStatusResponse.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.GetStatusResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetStatusResponse} message GetStatusResponse message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        GetStatusResponse.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes a GetStatusResponse message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.GetStatusResponse
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.GetStatusResponse} GetStatusResponse
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        GetStatusResponse.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.GetStatusResponse();
                            while (reader.pos < end) {
                                var tag = reader.uint32();
                                switch (tag >>> 3) {
                                case 1:
                                    message.version = $root.org.dash.platform.dapi.v0.GetStatusResponse.Version.decode(reader, reader.uint32());
                                    break;
                                case 2:
                                    message.time = $root.org.dash.platform.dapi.v0.GetStatusResponse.Time.decode(reader, reader.uint32());
                                    break;
                                case 3:
                                    message.status = reader.int32();
                                    break;
                                case 4:
                                    message.syncProgress = reader.double();
                                    break;
                                case 5:
                                    message.chain = $root.org.dash.platform.dapi.v0.GetStatusResponse.Chain.decode(reader, reader.uint32());
                                    break;
                                case 6:
                                    message.masternode = $root.org.dash.platform.dapi.v0.GetStatusResponse.Masternode.decode(reader, reader.uint32());
                                    break;
                                case 7:
                                    message.network = $root.org.dash.platform.dapi.v0.GetStatusResponse.Network.decode(reader, reader.uint32());
                                    break;
                                default:
                                    reader.skipType(tag & 7);
                                    break;
                                }
                            }
                            return message;
                        };

                        /**
                         * Decodes a GetStatusResponse message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.GetStatusResponse
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.GetStatusResponse} GetStatusResponse
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        GetStatusResponse.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies a GetStatusResponse message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.GetStatusResponse
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        GetStatusResponse.verify = function verify(message) {
                            if (typeof message !== "object" || message === null)
                                return "object expected";
                            if (message.version != null && message.hasOwnProperty("version")) {
                                var error = $root.org.dash.platform.dapi.v0.GetStatusResponse.Version.verify(message.version);
                                if (error)
                                    return "version." + error;
                            }
                            if (message.time != null && message.hasOwnProperty("time")) {
                                var error = $root.org.dash.platform.dapi.v0.GetStatusResponse.Time.verify(message.time);
                                if (error)
                                    return "time." + error;
                            }
                            if (message.status != null && message.hasOwnProperty("status"))
                                switch (message.status) {
                                default:
                                    return "status: enum value expected";
                                case 0:
                                case 1:
                                case 2:
                                case 3:
                                    break;
                                }
                            if (message.syncProgress != null && message.hasOwnProperty("syncProgress"))
                                if (typeof message.syncProgress !== "number")
                                    return "syncProgress: number expected";
                            if (message.chain != null && message.hasOwnProperty("chain")) {
                                var error = $root.org.dash.platform.dapi.v0.GetStatusResponse.Chain.verify(message.chain);
                                if (error)
                                    return "chain." + error;
                            }
                            if (message.masternode != null && message.hasOwnProperty("masternode")) {
                                var error = $root.org.dash.platform.dapi.v0.GetStatusResponse.Masternode.verify(message.masternode);
                                if (error)
                                    return "masternode." + error;
                            }
                            if (message.network != null && message.hasOwnProperty("network")) {
                                var error = $root.org.dash.platform.dapi.v0.GetStatusResponse.Network.verify(message.network);
                                if (error)
                                    return "network." + error;
                            }
                            return null;
                        };

                        /**
                         * Creates a GetStatusResponse message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.GetStatusResponse
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.GetStatusResponse} GetStatusResponse
                         */
                        GetStatusResponse.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.GetStatusResponse)
                                return object;
                            var message = new $root.org.dash.platform.dapi.v0.GetStatusResponse();
                            if (object.version != null) {
                                if (typeof object.version !== "object")
                                    throw TypeError(".org.dash.platform.dapi.v0.GetStatusResponse.version: object expected");
                                message.version = $root.org.dash.platform.dapi.v0.GetStatusResponse.Version.fromObject(object.version);
                            }
                            if (object.time != null) {
                                if (typeof object.time !== "object")
                                    throw TypeError(".org.dash.platform.dapi.v0.GetStatusResponse.time: object expected");
                                message.time = $root.org.dash.platform.dapi.v0.GetStatusResponse.Time.fromObject(object.time);
                            }
                            switch (object.status) {
                            case "NOT_STARTED":
                            case 0:
                                message.status = 0;
                                break;
                            case "SYNCING":
                            case 1:
                                message.status = 1;
                                break;
                            case "READY":
                            case 2:
                                message.status = 2;
                                break;
                            case "ERROR":
                            case 3:
                                message.status = 3;
                                break;
                            }
                            if (object.syncProgress != null)
                                message.syncProgress = Number(object.syncProgress);
                            if (object.chain != null) {
                                if (typeof object.chain !== "object")
                                    throw TypeError(".org.dash.platform.dapi.v0.GetStatusResponse.chain: object expected");
                                message.chain = $root.org.dash.platform.dapi.v0.GetStatusResponse.Chain.fromObject(object.chain);
                            }
                            if (object.masternode != null) {
                                if (typeof object.masternode !== "object")
                                    throw TypeError(".org.dash.platform.dapi.v0.GetStatusResponse.masternode: object expected");
                                message.masternode = $root.org.dash.platform.dapi.v0.GetStatusResponse.Masternode.fromObject(object.masternode);
                            }
                            if (object.network != null) {
                                if (typeof object.network !== "object")
                                    throw TypeError(".org.dash.platform.dapi.v0.GetStatusResponse.network: object expected");
                                message.network = $root.org.dash.platform.dapi.v0.GetStatusResponse.Network.fromObject(object.network);
                            }
                            return message;
                        };

                        /**
                         * Creates a plain object from a GetStatusResponse message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.GetStatusResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.GetStatusResponse} message GetStatusResponse
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        GetStatusResponse.toObject = function toObject(message, options) {
                            if (!options)
                                options = {};
                            var object = {};
                            if (options.defaults) {
                                object.version = null;
                                object.time = null;
                                object.status = options.enums === String ? "NOT_STARTED" : 0;
                                object.syncProgress = 0;
                                object.chain = null;
                                object.masternode = null;
                                object.network = null;
                            }
                            if (message.version != null && message.hasOwnProperty("version"))
                                object.version = $root.org.dash.platform.dapi.v0.GetStatusResponse.Version.toObject(message.version, options);
                            if (message.time != null && message.hasOwnProperty("time"))
                                object.time = $root.org.dash.platform.dapi.v0.GetStatusResponse.Time.toObject(message.time, options);
                            if (message.status != null && message.hasOwnProperty("status"))
                                object.status = options.enums === String ? $root.org.dash.platform.dapi.v0.GetStatusResponse.Status[message.status] : message.status;
                            if (message.syncProgress != null && message.hasOwnProperty("syncProgress"))
                                object.syncProgress = options.json && !isFinite(message.syncProgress) ? String(message.syncProgress) : message.syncProgress;
                            if (message.chain != null && message.hasOwnProperty("chain"))
                                object.chain = $root.org.dash.platform.dapi.v0.GetStatusResponse.Chain.toObject(message.chain, options);
                            if (message.masternode != null && message.hasOwnProperty("masternode"))
                                object.masternode = $root.org.dash.platform.dapi.v0.GetStatusResponse.Masternode.toObject(message.masternode, options);
                            if (message.network != null && message.hasOwnProperty("network"))
                                object.network = $root.org.dash.platform.dapi.v0.GetStatusResponse.Network.toObject(message.network, options);
                            return object;
                        };

                        /**
                         * Converts this GetStatusResponse to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.GetStatusResponse
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        GetStatusResponse.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        GetStatusResponse.Version = (function() {

                            /**
                             * Properties of a Version.
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse
                             * @interface IVersion
                             * @property {number|null} [protocol] Version protocol
                             * @property {number|null} [software] Version software
                             * @property {string|null} [agent] Version agent
                             */

                            /**
                             * Constructs a new Version.
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse
                             * @classdesc Represents a Version.
                             * @implements IVersion
                             * @constructor
                             * @param {org.dash.platform.dapi.v0.GetStatusResponse.IVersion=} [properties] Properties to set
                             */
                            function Version(properties) {
                                if (properties)
                                    for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                        if (properties[keys[i]] != null)
                                            this[keys[i]] = properties[keys[i]];
                            }

                            /**
                             * Version protocol.
                             * @member {number} protocol
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.Version
                             * @instance
                             */
                            Version.prototype.protocol = 0;

                            /**
                             * Version software.
                             * @member {number} software
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.Version
                             * @instance
                             */
                            Version.prototype.software = 0;

                            /**
                             * Version agent.
                             * @member {string} agent
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.Version
                             * @instance
                             */
                            Version.prototype.agent = "";

                            /**
                             * Creates a new Version instance using the specified properties.
                             * @function create
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.Version
                             * @static
                             * @param {org.dash.platform.dapi.v0.GetStatusResponse.IVersion=} [properties] Properties to set
                             * @returns {org.dash.platform.dapi.v0.GetStatusResponse.Version} Version instance
                             */
                            Version.create = function create(properties) {
                                return new Version(properties);
                            };

                            /**
                             * Encodes the specified Version message. Does not implicitly {@link org.dash.platform.dapi.v0.GetStatusResponse.Version.verify|verify} messages.
                             * @function encode
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.Version
                             * @static
                             * @param {org.dash.platform.dapi.v0.GetStatusResponse.IVersion} message Version message or plain object to encode
                             * @param {$protobuf.Writer} [writer] Writer to encode to
                             * @returns {$protobuf.Writer} Writer
                             */
                            Version.encode = function encode(message, writer) {
                                if (!writer)
                                    writer = $Writer.create();
                                if (message.protocol != null && Object.hasOwnProperty.call(message, "protocol"))
                                    writer.uint32(/* id 1, wireType 0 =*/8).uint32(message.protocol);
                                if (message.software != null && Object.hasOwnProperty.call(message, "software"))
                                    writer.uint32(/* id 2, wireType 0 =*/16).uint32(message.software);
                                if (message.agent != null && Object.hasOwnProperty.call(message, "agent"))
                                    writer.uint32(/* id 3, wireType 2 =*/26).string(message.agent);
                                return writer;
                            };

                            /**
                             * Encodes the specified Version message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.GetStatusResponse.Version.verify|verify} messages.
                             * @function encodeDelimited
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.Version
                             * @static
                             * @param {org.dash.platform.dapi.v0.GetStatusResponse.IVersion} message Version message or plain object to encode
                             * @param {$protobuf.Writer} [writer] Writer to encode to
                             * @returns {$protobuf.Writer} Writer
                             */
                            Version.encodeDelimited = function encodeDelimited(message, writer) {
                                return this.encode(message, writer).ldelim();
                            };

                            /**
                             * Decodes a Version message from the specified reader or buffer.
                             * @function decode
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.Version
                             * @static
                             * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                             * @param {number} [length] Message length if known beforehand
                             * @returns {org.dash.platform.dapi.v0.GetStatusResponse.Version} Version
                             * @throws {Error} If the payload is not a reader or valid buffer
                             * @throws {$protobuf.util.ProtocolError} If required fields are missing
                             */
                            Version.decode = function decode(reader, length) {
                                if (!(reader instanceof $Reader))
                                    reader = $Reader.create(reader);
                                var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.GetStatusResponse.Version();
                                while (reader.pos < end) {
                                    var tag = reader.uint32();
                                    switch (tag >>> 3) {
                                    case 1:
                                        message.protocol = reader.uint32();
                                        break;
                                    case 2:
                                        message.software = reader.uint32();
                                        break;
                                    case 3:
                                        message.agent = reader.string();
                                        break;
                                    default:
                                        reader.skipType(tag & 7);
                                        break;
                                    }
                                }
                                return message;
                            };

                            /**
                             * Decodes a Version message from the specified reader or buffer, length delimited.
                             * @function decodeDelimited
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.Version
                             * @static
                             * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                             * @returns {org.dash.platform.dapi.v0.GetStatusResponse.Version} Version
                             * @throws {Error} If the payload is not a reader or valid buffer
                             * @throws {$protobuf.util.ProtocolError} If required fields are missing
                             */
                            Version.decodeDelimited = function decodeDelimited(reader) {
                                if (!(reader instanceof $Reader))
                                    reader = new $Reader(reader);
                                return this.decode(reader, reader.uint32());
                            };

                            /**
                             * Verifies a Version message.
                             * @function verify
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.Version
                             * @static
                             * @param {Object.<string,*>} message Plain object to verify
                             * @returns {string|null} `null` if valid, otherwise the reason why it is not
                             */
                            Version.verify = function verify(message) {
                                if (typeof message !== "object" || message === null)
                                    return "object expected";
                                if (message.protocol != null && message.hasOwnProperty("protocol"))
                                    if (!$util.isInteger(message.protocol))
                                        return "protocol: integer expected";
                                if (message.software != null && message.hasOwnProperty("software"))
                                    if (!$util.isInteger(message.software))
                                        return "software: integer expected";
                                if (message.agent != null && message.hasOwnProperty("agent"))
                                    if (!$util.isString(message.agent))
                                        return "agent: string expected";
                                return null;
                            };

                            /**
                             * Creates a Version message from a plain object. Also converts values to their respective internal types.
                             * @function fromObject
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.Version
                             * @static
                             * @param {Object.<string,*>} object Plain object
                             * @returns {org.dash.platform.dapi.v0.GetStatusResponse.Version} Version
                             */
                            Version.fromObject = function fromObject(object) {
                                if (object instanceof $root.org.dash.platform.dapi.v0.GetStatusResponse.Version)
                                    return object;
                                var message = new $root.org.dash.platform.dapi.v0.GetStatusResponse.Version();
                                if (object.protocol != null)
                                    message.protocol = object.protocol >>> 0;
                                if (object.software != null)
                                    message.software = object.software >>> 0;
                                if (object.agent != null)
                                    message.agent = String(object.agent);
                                return message;
                            };

                            /**
                             * Creates a plain object from a Version message. Also converts values to other types if specified.
                             * @function toObject
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.Version
                             * @static
                             * @param {org.dash.platform.dapi.v0.GetStatusResponse.Version} message Version
                             * @param {$protobuf.IConversionOptions} [options] Conversion options
                             * @returns {Object.<string,*>} Plain object
                             */
                            Version.toObject = function toObject(message, options) {
                                if (!options)
                                    options = {};
                                var object = {};
                                if (options.defaults) {
                                    object.protocol = 0;
                                    object.software = 0;
                                    object.agent = "";
                                }
                                if (message.protocol != null && message.hasOwnProperty("protocol"))
                                    object.protocol = message.protocol;
                                if (message.software != null && message.hasOwnProperty("software"))
                                    object.software = message.software;
                                if (message.agent != null && message.hasOwnProperty("agent"))
                                    object.agent = message.agent;
                                return object;
                            };

                            /**
                             * Converts this Version to JSON.
                             * @function toJSON
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.Version
                             * @instance
                             * @returns {Object.<string,*>} JSON object
                             */
                            Version.prototype.toJSON = function toJSON() {
                                return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                            };

                            return Version;
                        })();

                        GetStatusResponse.Time = (function() {

                            /**
                             * Properties of a Time.
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse
                             * @interface ITime
                             * @property {number|null} [now] Time now
                             * @property {number|null} [offset] Time offset
                             * @property {number|null} [median] Time median
                             */

                            /**
                             * Constructs a new Time.
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse
                             * @classdesc Represents a Time.
                             * @implements ITime
                             * @constructor
                             * @param {org.dash.platform.dapi.v0.GetStatusResponse.ITime=} [properties] Properties to set
                             */
                            function Time(properties) {
                                if (properties)
                                    for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                        if (properties[keys[i]] != null)
                                            this[keys[i]] = properties[keys[i]];
                            }

                            /**
                             * Time now.
                             * @member {number} now
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.Time
                             * @instance
                             */
                            Time.prototype.now = 0;

                            /**
                             * Time offset.
                             * @member {number} offset
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.Time
                             * @instance
                             */
                            Time.prototype.offset = 0;

                            /**
                             * Time median.
                             * @member {number} median
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.Time
                             * @instance
                             */
                            Time.prototype.median = 0;

                            /**
                             * Creates a new Time instance using the specified properties.
                             * @function create
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.Time
                             * @static
                             * @param {org.dash.platform.dapi.v0.GetStatusResponse.ITime=} [properties] Properties to set
                             * @returns {org.dash.platform.dapi.v0.GetStatusResponse.Time} Time instance
                             */
                            Time.create = function create(properties) {
                                return new Time(properties);
                            };

                            /**
                             * Encodes the specified Time message. Does not implicitly {@link org.dash.platform.dapi.v0.GetStatusResponse.Time.verify|verify} messages.
                             * @function encode
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.Time
                             * @static
                             * @param {org.dash.platform.dapi.v0.GetStatusResponse.ITime} message Time message or plain object to encode
                             * @param {$protobuf.Writer} [writer] Writer to encode to
                             * @returns {$protobuf.Writer} Writer
                             */
                            Time.encode = function encode(message, writer) {
                                if (!writer)
                                    writer = $Writer.create();
                                if (message.now != null && Object.hasOwnProperty.call(message, "now"))
                                    writer.uint32(/* id 1, wireType 0 =*/8).uint32(message.now);
                                if (message.offset != null && Object.hasOwnProperty.call(message, "offset"))
                                    writer.uint32(/* id 2, wireType 0 =*/16).int32(message.offset);
                                if (message.median != null && Object.hasOwnProperty.call(message, "median"))
                                    writer.uint32(/* id 3, wireType 0 =*/24).uint32(message.median);
                                return writer;
                            };

                            /**
                             * Encodes the specified Time message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.GetStatusResponse.Time.verify|verify} messages.
                             * @function encodeDelimited
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.Time
                             * @static
                             * @param {org.dash.platform.dapi.v0.GetStatusResponse.ITime} message Time message or plain object to encode
                             * @param {$protobuf.Writer} [writer] Writer to encode to
                             * @returns {$protobuf.Writer} Writer
                             */
                            Time.encodeDelimited = function encodeDelimited(message, writer) {
                                return this.encode(message, writer).ldelim();
                            };

                            /**
                             * Decodes a Time message from the specified reader or buffer.
                             * @function decode
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.Time
                             * @static
                             * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                             * @param {number} [length] Message length if known beforehand
                             * @returns {org.dash.platform.dapi.v0.GetStatusResponse.Time} Time
                             * @throws {Error} If the payload is not a reader or valid buffer
                             * @throws {$protobuf.util.ProtocolError} If required fields are missing
                             */
                            Time.decode = function decode(reader, length) {
                                if (!(reader instanceof $Reader))
                                    reader = $Reader.create(reader);
                                var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.GetStatusResponse.Time();
                                while (reader.pos < end) {
                                    var tag = reader.uint32();
                                    switch (tag >>> 3) {
                                    case 1:
                                        message.now = reader.uint32();
                                        break;
                                    case 2:
                                        message.offset = reader.int32();
                                        break;
                                    case 3:
                                        message.median = reader.uint32();
                                        break;
                                    default:
                                        reader.skipType(tag & 7);
                                        break;
                                    }
                                }
                                return message;
                            };

                            /**
                             * Decodes a Time message from the specified reader or buffer, length delimited.
                             * @function decodeDelimited
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.Time
                             * @static
                             * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                             * @returns {org.dash.platform.dapi.v0.GetStatusResponse.Time} Time
                             * @throws {Error} If the payload is not a reader or valid buffer
                             * @throws {$protobuf.util.ProtocolError} If required fields are missing
                             */
                            Time.decodeDelimited = function decodeDelimited(reader) {
                                if (!(reader instanceof $Reader))
                                    reader = new $Reader(reader);
                                return this.decode(reader, reader.uint32());
                            };

                            /**
                             * Verifies a Time message.
                             * @function verify
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.Time
                             * @static
                             * @param {Object.<string,*>} message Plain object to verify
                             * @returns {string|null} `null` if valid, otherwise the reason why it is not
                             */
                            Time.verify = function verify(message) {
                                if (typeof message !== "object" || message === null)
                                    return "object expected";
                                if (message.now != null && message.hasOwnProperty("now"))
                                    if (!$util.isInteger(message.now))
                                        return "now: integer expected";
                                if (message.offset != null && message.hasOwnProperty("offset"))
                                    if (!$util.isInteger(message.offset))
                                        return "offset: integer expected";
                                if (message.median != null && message.hasOwnProperty("median"))
                                    if (!$util.isInteger(message.median))
                                        return "median: integer expected";
                                return null;
                            };

                            /**
                             * Creates a Time message from a plain object. Also converts values to their respective internal types.
                             * @function fromObject
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.Time
                             * @static
                             * @param {Object.<string,*>} object Plain object
                             * @returns {org.dash.platform.dapi.v0.GetStatusResponse.Time} Time
                             */
                            Time.fromObject = function fromObject(object) {
                                if (object instanceof $root.org.dash.platform.dapi.v0.GetStatusResponse.Time)
                                    return object;
                                var message = new $root.org.dash.platform.dapi.v0.GetStatusResponse.Time();
                                if (object.now != null)
                                    message.now = object.now >>> 0;
                                if (object.offset != null)
                                    message.offset = object.offset | 0;
                                if (object.median != null)
                                    message.median = object.median >>> 0;
                                return message;
                            };

                            /**
                             * Creates a plain object from a Time message. Also converts values to other types if specified.
                             * @function toObject
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.Time
                             * @static
                             * @param {org.dash.platform.dapi.v0.GetStatusResponse.Time} message Time
                             * @param {$protobuf.IConversionOptions} [options] Conversion options
                             * @returns {Object.<string,*>} Plain object
                             */
                            Time.toObject = function toObject(message, options) {
                                if (!options)
                                    options = {};
                                var object = {};
                                if (options.defaults) {
                                    object.now = 0;
                                    object.offset = 0;
                                    object.median = 0;
                                }
                                if (message.now != null && message.hasOwnProperty("now"))
                                    object.now = message.now;
                                if (message.offset != null && message.hasOwnProperty("offset"))
                                    object.offset = message.offset;
                                if (message.median != null && message.hasOwnProperty("median"))
                                    object.median = message.median;
                                return object;
                            };

                            /**
                             * Converts this Time to JSON.
                             * @function toJSON
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.Time
                             * @instance
                             * @returns {Object.<string,*>} JSON object
                             */
                            Time.prototype.toJSON = function toJSON() {
                                return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                            };

                            return Time;
                        })();

                        /**
                         * Status enum.
                         * @name org.dash.platform.dapi.v0.GetStatusResponse.Status
                         * @enum {number}
                         * @property {number} NOT_STARTED=0 NOT_STARTED value
                         * @property {number} SYNCING=1 SYNCING value
                         * @property {number} READY=2 READY value
                         * @property {number} ERROR=3 ERROR value
                         */
                        GetStatusResponse.Status = (function() {
                            var valuesById = {}, values = Object.create(valuesById);
                            values[valuesById[0] = "NOT_STARTED"] = 0;
                            values[valuesById[1] = "SYNCING"] = 1;
                            values[valuesById[2] = "READY"] = 2;
                            values[valuesById[3] = "ERROR"] = 3;
                            return values;
                        })();

                        GetStatusResponse.Chain = (function() {

                            /**
                             * Properties of a Chain.
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse
                             * @interface IChain
                             * @property {string|null} [name] Chain name
                             * @property {number|null} [headersCount] Chain headersCount
                             * @property {number|null} [blocksCount] Chain blocksCount
                             * @property {Uint8Array|null} [bestBlockHash] Chain bestBlockHash
                             * @property {number|null} [difficulty] Chain difficulty
                             * @property {Uint8Array|null} [chainWork] Chain chainWork
                             * @property {boolean|null} [isSynced] Chain isSynced
                             * @property {number|null} [syncProgress] Chain syncProgress
                             */

                            /**
                             * Constructs a new Chain.
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse
                             * @classdesc Represents a Chain.
                             * @implements IChain
                             * @constructor
                             * @param {org.dash.platform.dapi.v0.GetStatusResponse.IChain=} [properties] Properties to set
                             */
                            function Chain(properties) {
                                if (properties)
                                    for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                        if (properties[keys[i]] != null)
                                            this[keys[i]] = properties[keys[i]];
                            }

                            /**
                             * Chain name.
                             * @member {string} name
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.Chain
                             * @instance
                             */
                            Chain.prototype.name = "";

                            /**
                             * Chain headersCount.
                             * @member {number} headersCount
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.Chain
                             * @instance
                             */
                            Chain.prototype.headersCount = 0;

                            /**
                             * Chain blocksCount.
                             * @member {number} blocksCount
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.Chain
                             * @instance
                             */
                            Chain.prototype.blocksCount = 0;

                            /**
                             * Chain bestBlockHash.
                             * @member {Uint8Array} bestBlockHash
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.Chain
                             * @instance
                             */
                            Chain.prototype.bestBlockHash = $util.newBuffer([]);

                            /**
                             * Chain difficulty.
                             * @member {number} difficulty
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.Chain
                             * @instance
                             */
                            Chain.prototype.difficulty = 0;

                            /**
                             * Chain chainWork.
                             * @member {Uint8Array} chainWork
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.Chain
                             * @instance
                             */
                            Chain.prototype.chainWork = $util.newBuffer([]);

                            /**
                             * Chain isSynced.
                             * @member {boolean} isSynced
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.Chain
                             * @instance
                             */
                            Chain.prototype.isSynced = false;

                            /**
                             * Chain syncProgress.
                             * @member {number} syncProgress
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.Chain
                             * @instance
                             */
                            Chain.prototype.syncProgress = 0;

                            /**
                             * Creates a new Chain instance using the specified properties.
                             * @function create
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.Chain
                             * @static
                             * @param {org.dash.platform.dapi.v0.GetStatusResponse.IChain=} [properties] Properties to set
                             * @returns {org.dash.platform.dapi.v0.GetStatusResponse.Chain} Chain instance
                             */
                            Chain.create = function create(properties) {
                                return new Chain(properties);
                            };

                            /**
                             * Encodes the specified Chain message. Does not implicitly {@link org.dash.platform.dapi.v0.GetStatusResponse.Chain.verify|verify} messages.
                             * @function encode
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.Chain
                             * @static
                             * @param {org.dash.platform.dapi.v0.GetStatusResponse.IChain} message Chain message or plain object to encode
                             * @param {$protobuf.Writer} [writer] Writer to encode to
                             * @returns {$protobuf.Writer} Writer
                             */
                            Chain.encode = function encode(message, writer) {
                                if (!writer)
                                    writer = $Writer.create();
                                if (message.name != null && Object.hasOwnProperty.call(message, "name"))
                                    writer.uint32(/* id 1, wireType 2 =*/10).string(message.name);
                                if (message.headersCount != null && Object.hasOwnProperty.call(message, "headersCount"))
                                    writer.uint32(/* id 2, wireType 0 =*/16).uint32(message.headersCount);
                                if (message.blocksCount != null && Object.hasOwnProperty.call(message, "blocksCount"))
                                    writer.uint32(/* id 3, wireType 0 =*/24).uint32(message.blocksCount);
                                if (message.bestBlockHash != null && Object.hasOwnProperty.call(message, "bestBlockHash"))
                                    writer.uint32(/* id 4, wireType 2 =*/34).bytes(message.bestBlockHash);
                                if (message.difficulty != null && Object.hasOwnProperty.call(message, "difficulty"))
                                    writer.uint32(/* id 5, wireType 1 =*/41).double(message.difficulty);
                                if (message.chainWork != null && Object.hasOwnProperty.call(message, "chainWork"))
                                    writer.uint32(/* id 6, wireType 2 =*/50).bytes(message.chainWork);
                                if (message.isSynced != null && Object.hasOwnProperty.call(message, "isSynced"))
                                    writer.uint32(/* id 7, wireType 0 =*/56).bool(message.isSynced);
                                if (message.syncProgress != null && Object.hasOwnProperty.call(message, "syncProgress"))
                                    writer.uint32(/* id 8, wireType 1 =*/65).double(message.syncProgress);
                                return writer;
                            };

                            /**
                             * Encodes the specified Chain message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.GetStatusResponse.Chain.verify|verify} messages.
                             * @function encodeDelimited
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.Chain
                             * @static
                             * @param {org.dash.platform.dapi.v0.GetStatusResponse.IChain} message Chain message or plain object to encode
                             * @param {$protobuf.Writer} [writer] Writer to encode to
                             * @returns {$protobuf.Writer} Writer
                             */
                            Chain.encodeDelimited = function encodeDelimited(message, writer) {
                                return this.encode(message, writer).ldelim();
                            };

                            /**
                             * Decodes a Chain message from the specified reader or buffer.
                             * @function decode
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.Chain
                             * @static
                             * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                             * @param {number} [length] Message length if known beforehand
                             * @returns {org.dash.platform.dapi.v0.GetStatusResponse.Chain} Chain
                             * @throws {Error} If the payload is not a reader or valid buffer
                             * @throws {$protobuf.util.ProtocolError} If required fields are missing
                             */
                            Chain.decode = function decode(reader, length) {
                                if (!(reader instanceof $Reader))
                                    reader = $Reader.create(reader);
                                var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.GetStatusResponse.Chain();
                                while (reader.pos < end) {
                                    var tag = reader.uint32();
                                    switch (tag >>> 3) {
                                    case 1:
                                        message.name = reader.string();
                                        break;
                                    case 2:
                                        message.headersCount = reader.uint32();
                                        break;
                                    case 3:
                                        message.blocksCount = reader.uint32();
                                        break;
                                    case 4:
                                        message.bestBlockHash = reader.bytes();
                                        break;
                                    case 5:
                                        message.difficulty = reader.double();
                                        break;
                                    case 6:
                                        message.chainWork = reader.bytes();
                                        break;
                                    case 7:
                                        message.isSynced = reader.bool();
                                        break;
                                    case 8:
                                        message.syncProgress = reader.double();
                                        break;
                                    default:
                                        reader.skipType(tag & 7);
                                        break;
                                    }
                                }
                                return message;
                            };

                            /**
                             * Decodes a Chain message from the specified reader or buffer, length delimited.
                             * @function decodeDelimited
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.Chain
                             * @static
                             * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                             * @returns {org.dash.platform.dapi.v0.GetStatusResponse.Chain} Chain
                             * @throws {Error} If the payload is not a reader or valid buffer
                             * @throws {$protobuf.util.ProtocolError} If required fields are missing
                             */
                            Chain.decodeDelimited = function decodeDelimited(reader) {
                                if (!(reader instanceof $Reader))
                                    reader = new $Reader(reader);
                                return this.decode(reader, reader.uint32());
                            };

                            /**
                             * Verifies a Chain message.
                             * @function verify
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.Chain
                             * @static
                             * @param {Object.<string,*>} message Plain object to verify
                             * @returns {string|null} `null` if valid, otherwise the reason why it is not
                             */
                            Chain.verify = function verify(message) {
                                if (typeof message !== "object" || message === null)
                                    return "object expected";
                                if (message.name != null && message.hasOwnProperty("name"))
                                    if (!$util.isString(message.name))
                                        return "name: string expected";
                                if (message.headersCount != null && message.hasOwnProperty("headersCount"))
                                    if (!$util.isInteger(message.headersCount))
                                        return "headersCount: integer expected";
                                if (message.blocksCount != null && message.hasOwnProperty("blocksCount"))
                                    if (!$util.isInteger(message.blocksCount))
                                        return "blocksCount: integer expected";
                                if (message.bestBlockHash != null && message.hasOwnProperty("bestBlockHash"))
                                    if (!(message.bestBlockHash && typeof message.bestBlockHash.length === "number" || $util.isString(message.bestBlockHash)))
                                        return "bestBlockHash: buffer expected";
                                if (message.difficulty != null && message.hasOwnProperty("difficulty"))
                                    if (typeof message.difficulty !== "number")
                                        return "difficulty: number expected";
                                if (message.chainWork != null && message.hasOwnProperty("chainWork"))
                                    if (!(message.chainWork && typeof message.chainWork.length === "number" || $util.isString(message.chainWork)))
                                        return "chainWork: buffer expected";
                                if (message.isSynced != null && message.hasOwnProperty("isSynced"))
                                    if (typeof message.isSynced !== "boolean")
                                        return "isSynced: boolean expected";
                                if (message.syncProgress != null && message.hasOwnProperty("syncProgress"))
                                    if (typeof message.syncProgress !== "number")
                                        return "syncProgress: number expected";
                                return null;
                            };

                            /**
                             * Creates a Chain message from a plain object. Also converts values to their respective internal types.
                             * @function fromObject
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.Chain
                             * @static
                             * @param {Object.<string,*>} object Plain object
                             * @returns {org.dash.platform.dapi.v0.GetStatusResponse.Chain} Chain
                             */
                            Chain.fromObject = function fromObject(object) {
                                if (object instanceof $root.org.dash.platform.dapi.v0.GetStatusResponse.Chain)
                                    return object;
                                var message = new $root.org.dash.platform.dapi.v0.GetStatusResponse.Chain();
                                if (object.name != null)
                                    message.name = String(object.name);
                                if (object.headersCount != null)
                                    message.headersCount = object.headersCount >>> 0;
                                if (object.blocksCount != null)
                                    message.blocksCount = object.blocksCount >>> 0;
                                if (object.bestBlockHash != null)
                                    if (typeof object.bestBlockHash === "string")
                                        $util.base64.decode(object.bestBlockHash, message.bestBlockHash = $util.newBuffer($util.base64.length(object.bestBlockHash)), 0);
                                    else if (object.bestBlockHash.length >= 0)
                                        message.bestBlockHash = object.bestBlockHash;
                                if (object.difficulty != null)
                                    message.difficulty = Number(object.difficulty);
                                if (object.chainWork != null)
                                    if (typeof object.chainWork === "string")
                                        $util.base64.decode(object.chainWork, message.chainWork = $util.newBuffer($util.base64.length(object.chainWork)), 0);
                                    else if (object.chainWork.length >= 0)
                                        message.chainWork = object.chainWork;
                                if (object.isSynced != null)
                                    message.isSynced = Boolean(object.isSynced);
                                if (object.syncProgress != null)
                                    message.syncProgress = Number(object.syncProgress);
                                return message;
                            };

                            /**
                             * Creates a plain object from a Chain message. Also converts values to other types if specified.
                             * @function toObject
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.Chain
                             * @static
                             * @param {org.dash.platform.dapi.v0.GetStatusResponse.Chain} message Chain
                             * @param {$protobuf.IConversionOptions} [options] Conversion options
                             * @returns {Object.<string,*>} Plain object
                             */
                            Chain.toObject = function toObject(message, options) {
                                if (!options)
                                    options = {};
                                var object = {};
                                if (options.defaults) {
                                    object.name = "";
                                    object.headersCount = 0;
                                    object.blocksCount = 0;
                                    if (options.bytes === String)
                                        object.bestBlockHash = "";
                                    else {
                                        object.bestBlockHash = [];
                                        if (options.bytes !== Array)
                                            object.bestBlockHash = $util.newBuffer(object.bestBlockHash);
                                    }
                                    object.difficulty = 0;
                                    if (options.bytes === String)
                                        object.chainWork = "";
                                    else {
                                        object.chainWork = [];
                                        if (options.bytes !== Array)
                                            object.chainWork = $util.newBuffer(object.chainWork);
                                    }
                                    object.isSynced = false;
                                    object.syncProgress = 0;
                                }
                                if (message.name != null && message.hasOwnProperty("name"))
                                    object.name = message.name;
                                if (message.headersCount != null && message.hasOwnProperty("headersCount"))
                                    object.headersCount = message.headersCount;
                                if (message.blocksCount != null && message.hasOwnProperty("blocksCount"))
                                    object.blocksCount = message.blocksCount;
                                if (message.bestBlockHash != null && message.hasOwnProperty("bestBlockHash"))
                                    object.bestBlockHash = options.bytes === String ? $util.base64.encode(message.bestBlockHash, 0, message.bestBlockHash.length) : options.bytes === Array ? Array.prototype.slice.call(message.bestBlockHash) : message.bestBlockHash;
                                if (message.difficulty != null && message.hasOwnProperty("difficulty"))
                                    object.difficulty = options.json && !isFinite(message.difficulty) ? String(message.difficulty) : message.difficulty;
                                if (message.chainWork != null && message.hasOwnProperty("chainWork"))
                                    object.chainWork = options.bytes === String ? $util.base64.encode(message.chainWork, 0, message.chainWork.length) : options.bytes === Array ? Array.prototype.slice.call(message.chainWork) : message.chainWork;
                                if (message.isSynced != null && message.hasOwnProperty("isSynced"))
                                    object.isSynced = message.isSynced;
                                if (message.syncProgress != null && message.hasOwnProperty("syncProgress"))
                                    object.syncProgress = options.json && !isFinite(message.syncProgress) ? String(message.syncProgress) : message.syncProgress;
                                return object;
                            };

                            /**
                             * Converts this Chain to JSON.
                             * @function toJSON
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.Chain
                             * @instance
                             * @returns {Object.<string,*>} JSON object
                             */
                            Chain.prototype.toJSON = function toJSON() {
                                return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                            };

                            return Chain;
                        })();

                        GetStatusResponse.Masternode = (function() {

                            /**
                             * Properties of a Masternode.
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse
                             * @interface IMasternode
                             * @property {org.dash.platform.dapi.v0.GetStatusResponse.Masternode.Status|null} [status] Masternode status
                             * @property {Uint8Array|null} [proTxHash] Masternode proTxHash
                             * @property {number|null} [posePenalty] Masternode posePenalty
                             * @property {boolean|null} [isSynced] Masternode isSynced
                             * @property {number|null} [syncProgress] Masternode syncProgress
                             */

                            /**
                             * Constructs a new Masternode.
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse
                             * @classdesc Represents a Masternode.
                             * @implements IMasternode
                             * @constructor
                             * @param {org.dash.platform.dapi.v0.GetStatusResponse.IMasternode=} [properties] Properties to set
                             */
                            function Masternode(properties) {
                                if (properties)
                                    for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                        if (properties[keys[i]] != null)
                                            this[keys[i]] = properties[keys[i]];
                            }

                            /**
                             * Masternode status.
                             * @member {org.dash.platform.dapi.v0.GetStatusResponse.Masternode.Status} status
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.Masternode
                             * @instance
                             */
                            Masternode.prototype.status = 0;

                            /**
                             * Masternode proTxHash.
                             * @member {Uint8Array} proTxHash
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.Masternode
                             * @instance
                             */
                            Masternode.prototype.proTxHash = $util.newBuffer([]);

                            /**
                             * Masternode posePenalty.
                             * @member {number} posePenalty
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.Masternode
                             * @instance
                             */
                            Masternode.prototype.posePenalty = 0;

                            /**
                             * Masternode isSynced.
                             * @member {boolean} isSynced
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.Masternode
                             * @instance
                             */
                            Masternode.prototype.isSynced = false;

                            /**
                             * Masternode syncProgress.
                             * @member {number} syncProgress
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.Masternode
                             * @instance
                             */
                            Masternode.prototype.syncProgress = 0;

                            /**
                             * Creates a new Masternode instance using the specified properties.
                             * @function create
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.Masternode
                             * @static
                             * @param {org.dash.platform.dapi.v0.GetStatusResponse.IMasternode=} [properties] Properties to set
                             * @returns {org.dash.platform.dapi.v0.GetStatusResponse.Masternode} Masternode instance
                             */
                            Masternode.create = function create(properties) {
                                return new Masternode(properties);
                            };

                            /**
                             * Encodes the specified Masternode message. Does not implicitly {@link org.dash.platform.dapi.v0.GetStatusResponse.Masternode.verify|verify} messages.
                             * @function encode
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.Masternode
                             * @static
                             * @param {org.dash.platform.dapi.v0.GetStatusResponse.IMasternode} message Masternode message or plain object to encode
                             * @param {$protobuf.Writer} [writer] Writer to encode to
                             * @returns {$protobuf.Writer} Writer
                             */
                            Masternode.encode = function encode(message, writer) {
                                if (!writer)
                                    writer = $Writer.create();
                                if (message.status != null && Object.hasOwnProperty.call(message, "status"))
                                    writer.uint32(/* id 1, wireType 0 =*/8).int32(message.status);
                                if (message.proTxHash != null && Object.hasOwnProperty.call(message, "proTxHash"))
                                    writer.uint32(/* id 2, wireType 2 =*/18).bytes(message.proTxHash);
                                if (message.posePenalty != null && Object.hasOwnProperty.call(message, "posePenalty"))
                                    writer.uint32(/* id 3, wireType 0 =*/24).uint32(message.posePenalty);
                                if (message.isSynced != null && Object.hasOwnProperty.call(message, "isSynced"))
                                    writer.uint32(/* id 4, wireType 0 =*/32).bool(message.isSynced);
                                if (message.syncProgress != null && Object.hasOwnProperty.call(message, "syncProgress"))
                                    writer.uint32(/* id 5, wireType 1 =*/41).double(message.syncProgress);
                                return writer;
                            };

                            /**
                             * Encodes the specified Masternode message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.GetStatusResponse.Masternode.verify|verify} messages.
                             * @function encodeDelimited
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.Masternode
                             * @static
                             * @param {org.dash.platform.dapi.v0.GetStatusResponse.IMasternode} message Masternode message or plain object to encode
                             * @param {$protobuf.Writer} [writer] Writer to encode to
                             * @returns {$protobuf.Writer} Writer
                             */
                            Masternode.encodeDelimited = function encodeDelimited(message, writer) {
                                return this.encode(message, writer).ldelim();
                            };

                            /**
                             * Decodes a Masternode message from the specified reader or buffer.
                             * @function decode
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.Masternode
                             * @static
                             * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                             * @param {number} [length] Message length if known beforehand
                             * @returns {org.dash.platform.dapi.v0.GetStatusResponse.Masternode} Masternode
                             * @throws {Error} If the payload is not a reader or valid buffer
                             * @throws {$protobuf.util.ProtocolError} If required fields are missing
                             */
                            Masternode.decode = function decode(reader, length) {
                                if (!(reader instanceof $Reader))
                                    reader = $Reader.create(reader);
                                var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.GetStatusResponse.Masternode();
                                while (reader.pos < end) {
                                    var tag = reader.uint32();
                                    switch (tag >>> 3) {
                                    case 1:
                                        message.status = reader.int32();
                                        break;
                                    case 2:
                                        message.proTxHash = reader.bytes();
                                        break;
                                    case 3:
                                        message.posePenalty = reader.uint32();
                                        break;
                                    case 4:
                                        message.isSynced = reader.bool();
                                        break;
                                    case 5:
                                        message.syncProgress = reader.double();
                                        break;
                                    default:
                                        reader.skipType(tag & 7);
                                        break;
                                    }
                                }
                                return message;
                            };

                            /**
                             * Decodes a Masternode message from the specified reader or buffer, length delimited.
                             * @function decodeDelimited
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.Masternode
                             * @static
                             * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                             * @returns {org.dash.platform.dapi.v0.GetStatusResponse.Masternode} Masternode
                             * @throws {Error} If the payload is not a reader or valid buffer
                             * @throws {$protobuf.util.ProtocolError} If required fields are missing
                             */
                            Masternode.decodeDelimited = function decodeDelimited(reader) {
                                if (!(reader instanceof $Reader))
                                    reader = new $Reader(reader);
                                return this.decode(reader, reader.uint32());
                            };

                            /**
                             * Verifies a Masternode message.
                             * @function verify
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.Masternode
                             * @static
                             * @param {Object.<string,*>} message Plain object to verify
                             * @returns {string|null} `null` if valid, otherwise the reason why it is not
                             */
                            Masternode.verify = function verify(message) {
                                if (typeof message !== "object" || message === null)
                                    return "object expected";
                                if (message.status != null && message.hasOwnProperty("status"))
                                    switch (message.status) {
                                    default:
                                        return "status: enum value expected";
                                    case 0:
                                    case 1:
                                    case 2:
                                    case 3:
                                    case 4:
                                    case 5:
                                    case 6:
                                    case 7:
                                        break;
                                    }
                                if (message.proTxHash != null && message.hasOwnProperty("proTxHash"))
                                    if (!(message.proTxHash && typeof message.proTxHash.length === "number" || $util.isString(message.proTxHash)))
                                        return "proTxHash: buffer expected";
                                if (message.posePenalty != null && message.hasOwnProperty("posePenalty"))
                                    if (!$util.isInteger(message.posePenalty))
                                        return "posePenalty: integer expected";
                                if (message.isSynced != null && message.hasOwnProperty("isSynced"))
                                    if (typeof message.isSynced !== "boolean")
                                        return "isSynced: boolean expected";
                                if (message.syncProgress != null && message.hasOwnProperty("syncProgress"))
                                    if (typeof message.syncProgress !== "number")
                                        return "syncProgress: number expected";
                                return null;
                            };

                            /**
                             * Creates a Masternode message from a plain object. Also converts values to their respective internal types.
                             * @function fromObject
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.Masternode
                             * @static
                             * @param {Object.<string,*>} object Plain object
                             * @returns {org.dash.platform.dapi.v0.GetStatusResponse.Masternode} Masternode
                             */
                            Masternode.fromObject = function fromObject(object) {
                                if (object instanceof $root.org.dash.platform.dapi.v0.GetStatusResponse.Masternode)
                                    return object;
                                var message = new $root.org.dash.platform.dapi.v0.GetStatusResponse.Masternode();
                                switch (object.status) {
                                case "UNKNOWN":
                                case 0:
                                    message.status = 0;
                                    break;
                                case "WAITING_FOR_PROTX":
                                case 1:
                                    message.status = 1;
                                    break;
                                case "POSE_BANNED":
                                case 2:
                                    message.status = 2;
                                    break;
                                case "REMOVED":
                                case 3:
                                    message.status = 3;
                                    break;
                                case "OPERATOR_KEY_CHANGED":
                                case 4:
                                    message.status = 4;
                                    break;
                                case "PROTX_IP_CHANGED":
                                case 5:
                                    message.status = 5;
                                    break;
                                case "READY":
                                case 6:
                                    message.status = 6;
                                    break;
                                case "ERROR":
                                case 7:
                                    message.status = 7;
                                    break;
                                }
                                if (object.proTxHash != null)
                                    if (typeof object.proTxHash === "string")
                                        $util.base64.decode(object.proTxHash, message.proTxHash = $util.newBuffer($util.base64.length(object.proTxHash)), 0);
                                    else if (object.proTxHash.length >= 0)
                                        message.proTxHash = object.proTxHash;
                                if (object.posePenalty != null)
                                    message.posePenalty = object.posePenalty >>> 0;
                                if (object.isSynced != null)
                                    message.isSynced = Boolean(object.isSynced);
                                if (object.syncProgress != null)
                                    message.syncProgress = Number(object.syncProgress);
                                return message;
                            };

                            /**
                             * Creates a plain object from a Masternode message. Also converts values to other types if specified.
                             * @function toObject
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.Masternode
                             * @static
                             * @param {org.dash.platform.dapi.v0.GetStatusResponse.Masternode} message Masternode
                             * @param {$protobuf.IConversionOptions} [options] Conversion options
                             * @returns {Object.<string,*>} Plain object
                             */
                            Masternode.toObject = function toObject(message, options) {
                                if (!options)
                                    options = {};
                                var object = {};
                                if (options.defaults) {
                                    object.status = options.enums === String ? "UNKNOWN" : 0;
                                    if (options.bytes === String)
                                        object.proTxHash = "";
                                    else {
                                        object.proTxHash = [];
                                        if (options.bytes !== Array)
                                            object.proTxHash = $util.newBuffer(object.proTxHash);
                                    }
                                    object.posePenalty = 0;
                                    object.isSynced = false;
                                    object.syncProgress = 0;
                                }
                                if (message.status != null && message.hasOwnProperty("status"))
                                    object.status = options.enums === String ? $root.org.dash.platform.dapi.v0.GetStatusResponse.Masternode.Status[message.status] : message.status;
                                if (message.proTxHash != null && message.hasOwnProperty("proTxHash"))
                                    object.proTxHash = options.bytes === String ? $util.base64.encode(message.proTxHash, 0, message.proTxHash.length) : options.bytes === Array ? Array.prototype.slice.call(message.proTxHash) : message.proTxHash;
                                if (message.posePenalty != null && message.hasOwnProperty("posePenalty"))
                                    object.posePenalty = message.posePenalty;
                                if (message.isSynced != null && message.hasOwnProperty("isSynced"))
                                    object.isSynced = message.isSynced;
                                if (message.syncProgress != null && message.hasOwnProperty("syncProgress"))
                                    object.syncProgress = options.json && !isFinite(message.syncProgress) ? String(message.syncProgress) : message.syncProgress;
                                return object;
                            };

                            /**
                             * Converts this Masternode to JSON.
                             * @function toJSON
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.Masternode
                             * @instance
                             * @returns {Object.<string,*>} JSON object
                             */
                            Masternode.prototype.toJSON = function toJSON() {
                                return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                            };

                            /**
                             * Status enum.
                             * @name org.dash.platform.dapi.v0.GetStatusResponse.Masternode.Status
                             * @enum {number}
                             * @property {number} UNKNOWN=0 UNKNOWN value
                             * @property {number} WAITING_FOR_PROTX=1 WAITING_FOR_PROTX value
                             * @property {number} POSE_BANNED=2 POSE_BANNED value
                             * @property {number} REMOVED=3 REMOVED value
                             * @property {number} OPERATOR_KEY_CHANGED=4 OPERATOR_KEY_CHANGED value
                             * @property {number} PROTX_IP_CHANGED=5 PROTX_IP_CHANGED value
                             * @property {number} READY=6 READY value
                             * @property {number} ERROR=7 ERROR value
                             */
                            Masternode.Status = (function() {
                                var valuesById = {}, values = Object.create(valuesById);
                                values[valuesById[0] = "UNKNOWN"] = 0;
                                values[valuesById[1] = "WAITING_FOR_PROTX"] = 1;
                                values[valuesById[2] = "POSE_BANNED"] = 2;
                                values[valuesById[3] = "REMOVED"] = 3;
                                values[valuesById[4] = "OPERATOR_KEY_CHANGED"] = 4;
                                values[valuesById[5] = "PROTX_IP_CHANGED"] = 5;
                                values[valuesById[6] = "READY"] = 6;
                                values[valuesById[7] = "ERROR"] = 7;
                                return values;
                            })();

                            return Masternode;
                        })();

                        GetStatusResponse.NetworkFee = (function() {

                            /**
                             * Properties of a NetworkFee.
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse
                             * @interface INetworkFee
                             * @property {number|null} [relay] NetworkFee relay
                             * @property {number|null} [incremental] NetworkFee incremental
                             */

                            /**
                             * Constructs a new NetworkFee.
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse
                             * @classdesc Represents a NetworkFee.
                             * @implements INetworkFee
                             * @constructor
                             * @param {org.dash.platform.dapi.v0.GetStatusResponse.INetworkFee=} [properties] Properties to set
                             */
                            function NetworkFee(properties) {
                                if (properties)
                                    for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                        if (properties[keys[i]] != null)
                                            this[keys[i]] = properties[keys[i]];
                            }

                            /**
                             * NetworkFee relay.
                             * @member {number} relay
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.NetworkFee
                             * @instance
                             */
                            NetworkFee.prototype.relay = 0;

                            /**
                             * NetworkFee incremental.
                             * @member {number} incremental
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.NetworkFee
                             * @instance
                             */
                            NetworkFee.prototype.incremental = 0;

                            /**
                             * Creates a new NetworkFee instance using the specified properties.
                             * @function create
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.NetworkFee
                             * @static
                             * @param {org.dash.platform.dapi.v0.GetStatusResponse.INetworkFee=} [properties] Properties to set
                             * @returns {org.dash.platform.dapi.v0.GetStatusResponse.NetworkFee} NetworkFee instance
                             */
                            NetworkFee.create = function create(properties) {
                                return new NetworkFee(properties);
                            };

                            /**
                             * Encodes the specified NetworkFee message. Does not implicitly {@link org.dash.platform.dapi.v0.GetStatusResponse.NetworkFee.verify|verify} messages.
                             * @function encode
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.NetworkFee
                             * @static
                             * @param {org.dash.platform.dapi.v0.GetStatusResponse.INetworkFee} message NetworkFee message or plain object to encode
                             * @param {$protobuf.Writer} [writer] Writer to encode to
                             * @returns {$protobuf.Writer} Writer
                             */
                            NetworkFee.encode = function encode(message, writer) {
                                if (!writer)
                                    writer = $Writer.create();
                                if (message.relay != null && Object.hasOwnProperty.call(message, "relay"))
                                    writer.uint32(/* id 1, wireType 1 =*/9).double(message.relay);
                                if (message.incremental != null && Object.hasOwnProperty.call(message, "incremental"))
                                    writer.uint32(/* id 2, wireType 1 =*/17).double(message.incremental);
                                return writer;
                            };

                            /**
                             * Encodes the specified NetworkFee message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.GetStatusResponse.NetworkFee.verify|verify} messages.
                             * @function encodeDelimited
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.NetworkFee
                             * @static
                             * @param {org.dash.platform.dapi.v0.GetStatusResponse.INetworkFee} message NetworkFee message or plain object to encode
                             * @param {$protobuf.Writer} [writer] Writer to encode to
                             * @returns {$protobuf.Writer} Writer
                             */
                            NetworkFee.encodeDelimited = function encodeDelimited(message, writer) {
                                return this.encode(message, writer).ldelim();
                            };

                            /**
                             * Decodes a NetworkFee message from the specified reader or buffer.
                             * @function decode
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.NetworkFee
                             * @static
                             * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                             * @param {number} [length] Message length if known beforehand
                             * @returns {org.dash.platform.dapi.v0.GetStatusResponse.NetworkFee} NetworkFee
                             * @throws {Error} If the payload is not a reader or valid buffer
                             * @throws {$protobuf.util.ProtocolError} If required fields are missing
                             */
                            NetworkFee.decode = function decode(reader, length) {
                                if (!(reader instanceof $Reader))
                                    reader = $Reader.create(reader);
                                var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.GetStatusResponse.NetworkFee();
                                while (reader.pos < end) {
                                    var tag = reader.uint32();
                                    switch (tag >>> 3) {
                                    case 1:
                                        message.relay = reader.double();
                                        break;
                                    case 2:
                                        message.incremental = reader.double();
                                        break;
                                    default:
                                        reader.skipType(tag & 7);
                                        break;
                                    }
                                }
                                return message;
                            };

                            /**
                             * Decodes a NetworkFee message from the specified reader or buffer, length delimited.
                             * @function decodeDelimited
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.NetworkFee
                             * @static
                             * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                             * @returns {org.dash.platform.dapi.v0.GetStatusResponse.NetworkFee} NetworkFee
                             * @throws {Error} If the payload is not a reader or valid buffer
                             * @throws {$protobuf.util.ProtocolError} If required fields are missing
                             */
                            NetworkFee.decodeDelimited = function decodeDelimited(reader) {
                                if (!(reader instanceof $Reader))
                                    reader = new $Reader(reader);
                                return this.decode(reader, reader.uint32());
                            };

                            /**
                             * Verifies a NetworkFee message.
                             * @function verify
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.NetworkFee
                             * @static
                             * @param {Object.<string,*>} message Plain object to verify
                             * @returns {string|null} `null` if valid, otherwise the reason why it is not
                             */
                            NetworkFee.verify = function verify(message) {
                                if (typeof message !== "object" || message === null)
                                    return "object expected";
                                if (message.relay != null && message.hasOwnProperty("relay"))
                                    if (typeof message.relay !== "number")
                                        return "relay: number expected";
                                if (message.incremental != null && message.hasOwnProperty("incremental"))
                                    if (typeof message.incremental !== "number")
                                        return "incremental: number expected";
                                return null;
                            };

                            /**
                             * Creates a NetworkFee message from a plain object. Also converts values to their respective internal types.
                             * @function fromObject
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.NetworkFee
                             * @static
                             * @param {Object.<string,*>} object Plain object
                             * @returns {org.dash.platform.dapi.v0.GetStatusResponse.NetworkFee} NetworkFee
                             */
                            NetworkFee.fromObject = function fromObject(object) {
                                if (object instanceof $root.org.dash.platform.dapi.v0.GetStatusResponse.NetworkFee)
                                    return object;
                                var message = new $root.org.dash.platform.dapi.v0.GetStatusResponse.NetworkFee();
                                if (object.relay != null)
                                    message.relay = Number(object.relay);
                                if (object.incremental != null)
                                    message.incremental = Number(object.incremental);
                                return message;
                            };

                            /**
                             * Creates a plain object from a NetworkFee message. Also converts values to other types if specified.
                             * @function toObject
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.NetworkFee
                             * @static
                             * @param {org.dash.platform.dapi.v0.GetStatusResponse.NetworkFee} message NetworkFee
                             * @param {$protobuf.IConversionOptions} [options] Conversion options
                             * @returns {Object.<string,*>} Plain object
                             */
                            NetworkFee.toObject = function toObject(message, options) {
                                if (!options)
                                    options = {};
                                var object = {};
                                if (options.defaults) {
                                    object.relay = 0;
                                    object.incremental = 0;
                                }
                                if (message.relay != null && message.hasOwnProperty("relay"))
                                    object.relay = options.json && !isFinite(message.relay) ? String(message.relay) : message.relay;
                                if (message.incremental != null && message.hasOwnProperty("incremental"))
                                    object.incremental = options.json && !isFinite(message.incremental) ? String(message.incremental) : message.incremental;
                                return object;
                            };

                            /**
                             * Converts this NetworkFee to JSON.
                             * @function toJSON
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.NetworkFee
                             * @instance
                             * @returns {Object.<string,*>} JSON object
                             */
                            NetworkFee.prototype.toJSON = function toJSON() {
                                return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                            };

                            return NetworkFee;
                        })();

                        GetStatusResponse.Network = (function() {

                            /**
                             * Properties of a Network.
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse
                             * @interface INetwork
                             * @property {number|null} [peersCount] Network peersCount
                             * @property {org.dash.platform.dapi.v0.GetStatusResponse.INetworkFee|null} [fee] Network fee
                             */

                            /**
                             * Constructs a new Network.
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse
                             * @classdesc Represents a Network.
                             * @implements INetwork
                             * @constructor
                             * @param {org.dash.platform.dapi.v0.GetStatusResponse.INetwork=} [properties] Properties to set
                             */
                            function Network(properties) {
                                if (properties)
                                    for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                        if (properties[keys[i]] != null)
                                            this[keys[i]] = properties[keys[i]];
                            }

                            /**
                             * Network peersCount.
                             * @member {number} peersCount
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.Network
                             * @instance
                             */
                            Network.prototype.peersCount = 0;

                            /**
                             * Network fee.
                             * @member {org.dash.platform.dapi.v0.GetStatusResponse.INetworkFee|null|undefined} fee
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.Network
                             * @instance
                             */
                            Network.prototype.fee = null;

                            /**
                             * Creates a new Network instance using the specified properties.
                             * @function create
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.Network
                             * @static
                             * @param {org.dash.platform.dapi.v0.GetStatusResponse.INetwork=} [properties] Properties to set
                             * @returns {org.dash.platform.dapi.v0.GetStatusResponse.Network} Network instance
                             */
                            Network.create = function create(properties) {
                                return new Network(properties);
                            };

                            /**
                             * Encodes the specified Network message. Does not implicitly {@link org.dash.platform.dapi.v0.GetStatusResponse.Network.verify|verify} messages.
                             * @function encode
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.Network
                             * @static
                             * @param {org.dash.platform.dapi.v0.GetStatusResponse.INetwork} message Network message or plain object to encode
                             * @param {$protobuf.Writer} [writer] Writer to encode to
                             * @returns {$protobuf.Writer} Writer
                             */
                            Network.encode = function encode(message, writer) {
                                if (!writer)
                                    writer = $Writer.create();
                                if (message.peersCount != null && Object.hasOwnProperty.call(message, "peersCount"))
                                    writer.uint32(/* id 1, wireType 0 =*/8).uint32(message.peersCount);
                                if (message.fee != null && Object.hasOwnProperty.call(message, "fee"))
                                    $root.org.dash.platform.dapi.v0.GetStatusResponse.NetworkFee.encode(message.fee, writer.uint32(/* id 2, wireType 2 =*/18).fork()).ldelim();
                                return writer;
                            };

                            /**
                             * Encodes the specified Network message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.GetStatusResponse.Network.verify|verify} messages.
                             * @function encodeDelimited
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.Network
                             * @static
                             * @param {org.dash.platform.dapi.v0.GetStatusResponse.INetwork} message Network message or plain object to encode
                             * @param {$protobuf.Writer} [writer] Writer to encode to
                             * @returns {$protobuf.Writer} Writer
                             */
                            Network.encodeDelimited = function encodeDelimited(message, writer) {
                                return this.encode(message, writer).ldelim();
                            };

                            /**
                             * Decodes a Network message from the specified reader or buffer.
                             * @function decode
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.Network
                             * @static
                             * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                             * @param {number} [length] Message length if known beforehand
                             * @returns {org.dash.platform.dapi.v0.GetStatusResponse.Network} Network
                             * @throws {Error} If the payload is not a reader or valid buffer
                             * @throws {$protobuf.util.ProtocolError} If required fields are missing
                             */
                            Network.decode = function decode(reader, length) {
                                if (!(reader instanceof $Reader))
                                    reader = $Reader.create(reader);
                                var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.GetStatusResponse.Network();
                                while (reader.pos < end) {
                                    var tag = reader.uint32();
                                    switch (tag >>> 3) {
                                    case 1:
                                        message.peersCount = reader.uint32();
                                        break;
                                    case 2:
                                        message.fee = $root.org.dash.platform.dapi.v0.GetStatusResponse.NetworkFee.decode(reader, reader.uint32());
                                        break;
                                    default:
                                        reader.skipType(tag & 7);
                                        break;
                                    }
                                }
                                return message;
                            };

                            /**
                             * Decodes a Network message from the specified reader or buffer, length delimited.
                             * @function decodeDelimited
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.Network
                             * @static
                             * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                             * @returns {org.dash.platform.dapi.v0.GetStatusResponse.Network} Network
                             * @throws {Error} If the payload is not a reader or valid buffer
                             * @throws {$protobuf.util.ProtocolError} If required fields are missing
                             */
                            Network.decodeDelimited = function decodeDelimited(reader) {
                                if (!(reader instanceof $Reader))
                                    reader = new $Reader(reader);
                                return this.decode(reader, reader.uint32());
                            };

                            /**
                             * Verifies a Network message.
                             * @function verify
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.Network
                             * @static
                             * @param {Object.<string,*>} message Plain object to verify
                             * @returns {string|null} `null` if valid, otherwise the reason why it is not
                             */
                            Network.verify = function verify(message) {
                                if (typeof message !== "object" || message === null)
                                    return "object expected";
                                if (message.peersCount != null && message.hasOwnProperty("peersCount"))
                                    if (!$util.isInteger(message.peersCount))
                                        return "peersCount: integer expected";
                                if (message.fee != null && message.hasOwnProperty("fee")) {
                                    var error = $root.org.dash.platform.dapi.v0.GetStatusResponse.NetworkFee.verify(message.fee);
                                    if (error)
                                        return "fee." + error;
                                }
                                return null;
                            };

                            /**
                             * Creates a Network message from a plain object. Also converts values to their respective internal types.
                             * @function fromObject
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.Network
                             * @static
                             * @param {Object.<string,*>} object Plain object
                             * @returns {org.dash.platform.dapi.v0.GetStatusResponse.Network} Network
                             */
                            Network.fromObject = function fromObject(object) {
                                if (object instanceof $root.org.dash.platform.dapi.v0.GetStatusResponse.Network)
                                    return object;
                                var message = new $root.org.dash.platform.dapi.v0.GetStatusResponse.Network();
                                if (object.peersCount != null)
                                    message.peersCount = object.peersCount >>> 0;
                                if (object.fee != null) {
                                    if (typeof object.fee !== "object")
                                        throw TypeError(".org.dash.platform.dapi.v0.GetStatusResponse.Network.fee: object expected");
                                    message.fee = $root.org.dash.platform.dapi.v0.GetStatusResponse.NetworkFee.fromObject(object.fee);
                                }
                                return message;
                            };

                            /**
                             * Creates a plain object from a Network message. Also converts values to other types if specified.
                             * @function toObject
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.Network
                             * @static
                             * @param {org.dash.platform.dapi.v0.GetStatusResponse.Network} message Network
                             * @param {$protobuf.IConversionOptions} [options] Conversion options
                             * @returns {Object.<string,*>} Plain object
                             */
                            Network.toObject = function toObject(message, options) {
                                if (!options)
                                    options = {};
                                var object = {};
                                if (options.defaults) {
                                    object.peersCount = 0;
                                    object.fee = null;
                                }
                                if (message.peersCount != null && message.hasOwnProperty("peersCount"))
                                    object.peersCount = message.peersCount;
                                if (message.fee != null && message.hasOwnProperty("fee"))
                                    object.fee = $root.org.dash.platform.dapi.v0.GetStatusResponse.NetworkFee.toObject(message.fee, options);
                                return object;
                            };

                            /**
                             * Converts this Network to JSON.
                             * @function toJSON
                             * @memberof org.dash.platform.dapi.v0.GetStatusResponse.Network
                             * @instance
                             * @returns {Object.<string,*>} JSON object
                             */
                            Network.prototype.toJSON = function toJSON() {
                                return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                            };

                            return Network;
                        })();

                        return GetStatusResponse;
                    })();

                    v0.GetBlockRequest = (function() {

                        /**
                         * Properties of a GetBlockRequest.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface IGetBlockRequest
                         * @property {number|null} [height] GetBlockRequest height
                         * @property {string|null} [hash] GetBlockRequest hash
                         */

                        /**
                         * Constructs a new GetBlockRequest.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a GetBlockRequest.
                         * @implements IGetBlockRequest
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.IGetBlockRequest=} [properties] Properties to set
                         */
                        function GetBlockRequest(properties) {
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * GetBlockRequest height.
                         * @member {number} height
                         * @memberof org.dash.platform.dapi.v0.GetBlockRequest
                         * @instance
                         */
                        GetBlockRequest.prototype.height = 0;

                        /**
                         * GetBlockRequest hash.
                         * @member {string} hash
                         * @memberof org.dash.platform.dapi.v0.GetBlockRequest
                         * @instance
                         */
                        GetBlockRequest.prototype.hash = "";

                        // OneOf field names bound to virtual getters and setters
                        var $oneOfFields;

                        /**
                         * GetBlockRequest block.
                         * @member {"height"|"hash"|undefined} block
                         * @memberof org.dash.platform.dapi.v0.GetBlockRequest
                         * @instance
                         */
                        Object.defineProperty(GetBlockRequest.prototype, "block", {
                            get: $util.oneOfGetter($oneOfFields = ["height", "hash"]),
                            set: $util.oneOfSetter($oneOfFields)
                        });

                        /**
                         * Creates a new GetBlockRequest instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.GetBlockRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetBlockRequest=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.GetBlockRequest} GetBlockRequest instance
                         */
                        GetBlockRequest.create = function create(properties) {
                            return new GetBlockRequest(properties);
                        };

                        /**
                         * Encodes the specified GetBlockRequest message. Does not implicitly {@link org.dash.platform.dapi.v0.GetBlockRequest.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.GetBlockRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetBlockRequest} message GetBlockRequest message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        GetBlockRequest.encode = function encode(message, writer) {
                            if (!writer)
                                writer = $Writer.create();
                            if (message.height != null && Object.hasOwnProperty.call(message, "height"))
                                writer.uint32(/* id 1, wireType 0 =*/8).uint32(message.height);
                            if (message.hash != null && Object.hasOwnProperty.call(message, "hash"))
                                writer.uint32(/* id 2, wireType 2 =*/18).string(message.hash);
                            return writer;
                        };

                        /**
                         * Encodes the specified GetBlockRequest message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.GetBlockRequest.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.GetBlockRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetBlockRequest} message GetBlockRequest message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        GetBlockRequest.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes a GetBlockRequest message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.GetBlockRequest
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.GetBlockRequest} GetBlockRequest
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        GetBlockRequest.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.GetBlockRequest();
                            while (reader.pos < end) {
                                var tag = reader.uint32();
                                switch (tag >>> 3) {
                                case 1:
                                    message.height = reader.uint32();
                                    break;
                                case 2:
                                    message.hash = reader.string();
                                    break;
                                default:
                                    reader.skipType(tag & 7);
                                    break;
                                }
                            }
                            return message;
                        };

                        /**
                         * Decodes a GetBlockRequest message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.GetBlockRequest
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.GetBlockRequest} GetBlockRequest
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        GetBlockRequest.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies a GetBlockRequest message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.GetBlockRequest
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        GetBlockRequest.verify = function verify(message) {
                            if (typeof message !== "object" || message === null)
                                return "object expected";
                            var properties = {};
                            if (message.height != null && message.hasOwnProperty("height")) {
                                properties.block = 1;
                                if (!$util.isInteger(message.height))
                                    return "height: integer expected";
                            }
                            if (message.hash != null && message.hasOwnProperty("hash")) {
                                if (properties.block === 1)
                                    return "block: multiple values";
                                properties.block = 1;
                                if (!$util.isString(message.hash))
                                    return "hash: string expected";
                            }
                            return null;
                        };

                        /**
                         * Creates a GetBlockRequest message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.GetBlockRequest
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.GetBlockRequest} GetBlockRequest
                         */
                        GetBlockRequest.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.GetBlockRequest)
                                return object;
                            var message = new $root.org.dash.platform.dapi.v0.GetBlockRequest();
                            if (object.height != null)
                                message.height = object.height >>> 0;
                            if (object.hash != null)
                                message.hash = String(object.hash);
                            return message;
                        };

                        /**
                         * Creates a plain object from a GetBlockRequest message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.GetBlockRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.GetBlockRequest} message GetBlockRequest
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        GetBlockRequest.toObject = function toObject(message, options) {
                            if (!options)
                                options = {};
                            var object = {};
                            if (message.height != null && message.hasOwnProperty("height")) {
                                object.height = message.height;
                                if (options.oneofs)
                                    object.block = "height";
                            }
                            if (message.hash != null && message.hasOwnProperty("hash")) {
                                object.hash = message.hash;
                                if (options.oneofs)
                                    object.block = "hash";
                            }
                            return object;
                        };

                        /**
                         * Converts this GetBlockRequest to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.GetBlockRequest
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        GetBlockRequest.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        return GetBlockRequest;
                    })();

                    v0.GetBlockResponse = (function() {

                        /**
                         * Properties of a GetBlockResponse.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface IGetBlockResponse
                         * @property {Uint8Array|null} [block] GetBlockResponse block
                         */

                        /**
                         * Constructs a new GetBlockResponse.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a GetBlockResponse.
                         * @implements IGetBlockResponse
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.IGetBlockResponse=} [properties] Properties to set
                         */
                        function GetBlockResponse(properties) {
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * GetBlockResponse block.
                         * @member {Uint8Array} block
                         * @memberof org.dash.platform.dapi.v0.GetBlockResponse
                         * @instance
                         */
                        GetBlockResponse.prototype.block = $util.newBuffer([]);

                        /**
                         * Creates a new GetBlockResponse instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.GetBlockResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetBlockResponse=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.GetBlockResponse} GetBlockResponse instance
                         */
                        GetBlockResponse.create = function create(properties) {
                            return new GetBlockResponse(properties);
                        };

                        /**
                         * Encodes the specified GetBlockResponse message. Does not implicitly {@link org.dash.platform.dapi.v0.GetBlockResponse.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.GetBlockResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetBlockResponse} message GetBlockResponse message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        GetBlockResponse.encode = function encode(message, writer) {
                            if (!writer)
                                writer = $Writer.create();
                            if (message.block != null && Object.hasOwnProperty.call(message, "block"))
                                writer.uint32(/* id 1, wireType 2 =*/10).bytes(message.block);
                            return writer;
                        };

                        /**
                         * Encodes the specified GetBlockResponse message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.GetBlockResponse.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.GetBlockResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetBlockResponse} message GetBlockResponse message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        GetBlockResponse.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes a GetBlockResponse message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.GetBlockResponse
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.GetBlockResponse} GetBlockResponse
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        GetBlockResponse.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.GetBlockResponse();
                            while (reader.pos < end) {
                                var tag = reader.uint32();
                                switch (tag >>> 3) {
                                case 1:
                                    message.block = reader.bytes();
                                    break;
                                default:
                                    reader.skipType(tag & 7);
                                    break;
                                }
                            }
                            return message;
                        };

                        /**
                         * Decodes a GetBlockResponse message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.GetBlockResponse
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.GetBlockResponse} GetBlockResponse
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        GetBlockResponse.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies a GetBlockResponse message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.GetBlockResponse
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        GetBlockResponse.verify = function verify(message) {
                            if (typeof message !== "object" || message === null)
                                return "object expected";
                            if (message.block != null && message.hasOwnProperty("block"))
                                if (!(message.block && typeof message.block.length === "number" || $util.isString(message.block)))
                                    return "block: buffer expected";
                            return null;
                        };

                        /**
                         * Creates a GetBlockResponse message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.GetBlockResponse
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.GetBlockResponse} GetBlockResponse
                         */
                        GetBlockResponse.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.GetBlockResponse)
                                return object;
                            var message = new $root.org.dash.platform.dapi.v0.GetBlockResponse();
                            if (object.block != null)
                                if (typeof object.block === "string")
                                    $util.base64.decode(object.block, message.block = $util.newBuffer($util.base64.length(object.block)), 0);
                                else if (object.block.length >= 0)
                                    message.block = object.block;
                            return message;
                        };

                        /**
                         * Creates a plain object from a GetBlockResponse message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.GetBlockResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.GetBlockResponse} message GetBlockResponse
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        GetBlockResponse.toObject = function toObject(message, options) {
                            if (!options)
                                options = {};
                            var object = {};
                            if (options.defaults)
                                if (options.bytes === String)
                                    object.block = "";
                                else {
                                    object.block = [];
                                    if (options.bytes !== Array)
                                        object.block = $util.newBuffer(object.block);
                                }
                            if (message.block != null && message.hasOwnProperty("block"))
                                object.block = options.bytes === String ? $util.base64.encode(message.block, 0, message.block.length) : options.bytes === Array ? Array.prototype.slice.call(message.block) : message.block;
                            return object;
                        };

                        /**
                         * Converts this GetBlockResponse to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.GetBlockResponse
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        GetBlockResponse.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        return GetBlockResponse;
                    })();

                    v0.BroadcastTransactionRequest = (function() {

                        /**
                         * Properties of a BroadcastTransactionRequest.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface IBroadcastTransactionRequest
                         * @property {Uint8Array|null} [transaction] BroadcastTransactionRequest transaction
                         * @property {boolean|null} [allowHighFees] BroadcastTransactionRequest allowHighFees
                         * @property {boolean|null} [bypassLimits] BroadcastTransactionRequest bypassLimits
                         */

                        /**
                         * Constructs a new BroadcastTransactionRequest.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a BroadcastTransactionRequest.
                         * @implements IBroadcastTransactionRequest
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.IBroadcastTransactionRequest=} [properties] Properties to set
                         */
                        function BroadcastTransactionRequest(properties) {
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * BroadcastTransactionRequest transaction.
                         * @member {Uint8Array} transaction
                         * @memberof org.dash.platform.dapi.v0.BroadcastTransactionRequest
                         * @instance
                         */
                        BroadcastTransactionRequest.prototype.transaction = $util.newBuffer([]);

                        /**
                         * BroadcastTransactionRequest allowHighFees.
                         * @member {boolean} allowHighFees
                         * @memberof org.dash.platform.dapi.v0.BroadcastTransactionRequest
                         * @instance
                         */
                        BroadcastTransactionRequest.prototype.allowHighFees = false;

                        /**
                         * BroadcastTransactionRequest bypassLimits.
                         * @member {boolean} bypassLimits
                         * @memberof org.dash.platform.dapi.v0.BroadcastTransactionRequest
                         * @instance
                         */
                        BroadcastTransactionRequest.prototype.bypassLimits = false;

                        /**
                         * Creates a new BroadcastTransactionRequest instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.BroadcastTransactionRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.IBroadcastTransactionRequest=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.BroadcastTransactionRequest} BroadcastTransactionRequest instance
                         */
                        BroadcastTransactionRequest.create = function create(properties) {
                            return new BroadcastTransactionRequest(properties);
                        };

                        /**
                         * Encodes the specified BroadcastTransactionRequest message. Does not implicitly {@link org.dash.platform.dapi.v0.BroadcastTransactionRequest.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.BroadcastTransactionRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.IBroadcastTransactionRequest} message BroadcastTransactionRequest message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        BroadcastTransactionRequest.encode = function encode(message, writer) {
                            if (!writer)
                                writer = $Writer.create();
                            if (message.transaction != null && Object.hasOwnProperty.call(message, "transaction"))
                                writer.uint32(/* id 1, wireType 2 =*/10).bytes(message.transaction);
                            if (message.allowHighFees != null && Object.hasOwnProperty.call(message, "allowHighFees"))
                                writer.uint32(/* id 2, wireType 0 =*/16).bool(message.allowHighFees);
                            if (message.bypassLimits != null && Object.hasOwnProperty.call(message, "bypassLimits"))
                                writer.uint32(/* id 3, wireType 0 =*/24).bool(message.bypassLimits);
                            return writer;
                        };

                        /**
                         * Encodes the specified BroadcastTransactionRequest message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.BroadcastTransactionRequest.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.BroadcastTransactionRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.IBroadcastTransactionRequest} message BroadcastTransactionRequest message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        BroadcastTransactionRequest.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes a BroadcastTransactionRequest message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.BroadcastTransactionRequest
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.BroadcastTransactionRequest} BroadcastTransactionRequest
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        BroadcastTransactionRequest.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.BroadcastTransactionRequest();
                            while (reader.pos < end) {
                                var tag = reader.uint32();
                                switch (tag >>> 3) {
                                case 1:
                                    message.transaction = reader.bytes();
                                    break;
                                case 2:
                                    message.allowHighFees = reader.bool();
                                    break;
                                case 3:
                                    message.bypassLimits = reader.bool();
                                    break;
                                default:
                                    reader.skipType(tag & 7);
                                    break;
                                }
                            }
                            return message;
                        };

                        /**
                         * Decodes a BroadcastTransactionRequest message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.BroadcastTransactionRequest
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.BroadcastTransactionRequest} BroadcastTransactionRequest
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        BroadcastTransactionRequest.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies a BroadcastTransactionRequest message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.BroadcastTransactionRequest
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        BroadcastTransactionRequest.verify = function verify(message) {
                            if (typeof message !== "object" || message === null)
                                return "object expected";
                            if (message.transaction != null && message.hasOwnProperty("transaction"))
                                if (!(message.transaction && typeof message.transaction.length === "number" || $util.isString(message.transaction)))
                                    return "transaction: buffer expected";
                            if (message.allowHighFees != null && message.hasOwnProperty("allowHighFees"))
                                if (typeof message.allowHighFees !== "boolean")
                                    return "allowHighFees: boolean expected";
                            if (message.bypassLimits != null && message.hasOwnProperty("bypassLimits"))
                                if (typeof message.bypassLimits !== "boolean")
                                    return "bypassLimits: boolean expected";
                            return null;
                        };

                        /**
                         * Creates a BroadcastTransactionRequest message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.BroadcastTransactionRequest
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.BroadcastTransactionRequest} BroadcastTransactionRequest
                         */
                        BroadcastTransactionRequest.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.BroadcastTransactionRequest)
                                return object;
                            var message = new $root.org.dash.platform.dapi.v0.BroadcastTransactionRequest();
                            if (object.transaction != null)
                                if (typeof object.transaction === "string")
                                    $util.base64.decode(object.transaction, message.transaction = $util.newBuffer($util.base64.length(object.transaction)), 0);
                                else if (object.transaction.length >= 0)
                                    message.transaction = object.transaction;
                            if (object.allowHighFees != null)
                                message.allowHighFees = Boolean(object.allowHighFees);
                            if (object.bypassLimits != null)
                                message.bypassLimits = Boolean(object.bypassLimits);
                            return message;
                        };

                        /**
                         * Creates a plain object from a BroadcastTransactionRequest message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.BroadcastTransactionRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.BroadcastTransactionRequest} message BroadcastTransactionRequest
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        BroadcastTransactionRequest.toObject = function toObject(message, options) {
                            if (!options)
                                options = {};
                            var object = {};
                            if (options.defaults) {
                                if (options.bytes === String)
                                    object.transaction = "";
                                else {
                                    object.transaction = [];
                                    if (options.bytes !== Array)
                                        object.transaction = $util.newBuffer(object.transaction);
                                }
                                object.allowHighFees = false;
                                object.bypassLimits = false;
                            }
                            if (message.transaction != null && message.hasOwnProperty("transaction"))
                                object.transaction = options.bytes === String ? $util.base64.encode(message.transaction, 0, message.transaction.length) : options.bytes === Array ? Array.prototype.slice.call(message.transaction) : message.transaction;
                            if (message.allowHighFees != null && message.hasOwnProperty("allowHighFees"))
                                object.allowHighFees = message.allowHighFees;
                            if (message.bypassLimits != null && message.hasOwnProperty("bypassLimits"))
                                object.bypassLimits = message.bypassLimits;
                            return object;
                        };

                        /**
                         * Converts this BroadcastTransactionRequest to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.BroadcastTransactionRequest
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        BroadcastTransactionRequest.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        return BroadcastTransactionRequest;
                    })();

                    v0.BroadcastTransactionResponse = (function() {

                        /**
                         * Properties of a BroadcastTransactionResponse.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface IBroadcastTransactionResponse
                         * @property {string|null} [transactionId] BroadcastTransactionResponse transactionId
                         */

                        /**
                         * Constructs a new BroadcastTransactionResponse.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a BroadcastTransactionResponse.
                         * @implements IBroadcastTransactionResponse
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.IBroadcastTransactionResponse=} [properties] Properties to set
                         */
                        function BroadcastTransactionResponse(properties) {
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * BroadcastTransactionResponse transactionId.
                         * @member {string} transactionId
                         * @memberof org.dash.platform.dapi.v0.BroadcastTransactionResponse
                         * @instance
                         */
                        BroadcastTransactionResponse.prototype.transactionId = "";

                        /**
                         * Creates a new BroadcastTransactionResponse instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.BroadcastTransactionResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.IBroadcastTransactionResponse=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.BroadcastTransactionResponse} BroadcastTransactionResponse instance
                         */
                        BroadcastTransactionResponse.create = function create(properties) {
                            return new BroadcastTransactionResponse(properties);
                        };

                        /**
                         * Encodes the specified BroadcastTransactionResponse message. Does not implicitly {@link org.dash.platform.dapi.v0.BroadcastTransactionResponse.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.BroadcastTransactionResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.IBroadcastTransactionResponse} message BroadcastTransactionResponse message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        BroadcastTransactionResponse.encode = function encode(message, writer) {
                            if (!writer)
                                writer = $Writer.create();
                            if (message.transactionId != null && Object.hasOwnProperty.call(message, "transactionId"))
                                writer.uint32(/* id 1, wireType 2 =*/10).string(message.transactionId);
                            return writer;
                        };

                        /**
                         * Encodes the specified BroadcastTransactionResponse message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.BroadcastTransactionResponse.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.BroadcastTransactionResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.IBroadcastTransactionResponse} message BroadcastTransactionResponse message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        BroadcastTransactionResponse.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes a BroadcastTransactionResponse message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.BroadcastTransactionResponse
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.BroadcastTransactionResponse} BroadcastTransactionResponse
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        BroadcastTransactionResponse.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.BroadcastTransactionResponse();
                            while (reader.pos < end) {
                                var tag = reader.uint32();
                                switch (tag >>> 3) {
                                case 1:
                                    message.transactionId = reader.string();
                                    break;
                                default:
                                    reader.skipType(tag & 7);
                                    break;
                                }
                            }
                            return message;
                        };

                        /**
                         * Decodes a BroadcastTransactionResponse message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.BroadcastTransactionResponse
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.BroadcastTransactionResponse} BroadcastTransactionResponse
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        BroadcastTransactionResponse.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies a BroadcastTransactionResponse message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.BroadcastTransactionResponse
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        BroadcastTransactionResponse.verify = function verify(message) {
                            if (typeof message !== "object" || message === null)
                                return "object expected";
                            if (message.transactionId != null && message.hasOwnProperty("transactionId"))
                                if (!$util.isString(message.transactionId))
                                    return "transactionId: string expected";
                            return null;
                        };

                        /**
                         * Creates a BroadcastTransactionResponse message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.BroadcastTransactionResponse
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.BroadcastTransactionResponse} BroadcastTransactionResponse
                         */
                        BroadcastTransactionResponse.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.BroadcastTransactionResponse)
                                return object;
                            var message = new $root.org.dash.platform.dapi.v0.BroadcastTransactionResponse();
                            if (object.transactionId != null)
                                message.transactionId = String(object.transactionId);
                            return message;
                        };

                        /**
                         * Creates a plain object from a BroadcastTransactionResponse message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.BroadcastTransactionResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.BroadcastTransactionResponse} message BroadcastTransactionResponse
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        BroadcastTransactionResponse.toObject = function toObject(message, options) {
                            if (!options)
                                options = {};
                            var object = {};
                            if (options.defaults)
                                object.transactionId = "";
                            if (message.transactionId != null && message.hasOwnProperty("transactionId"))
                                object.transactionId = message.transactionId;
                            return object;
                        };

                        /**
                         * Converts this BroadcastTransactionResponse to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.BroadcastTransactionResponse
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        BroadcastTransactionResponse.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        return BroadcastTransactionResponse;
                    })();

                    v0.GetTransactionRequest = (function() {

                        /**
                         * Properties of a GetTransactionRequest.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface IGetTransactionRequest
                         * @property {string|null} [id] GetTransactionRequest id
                         */

                        /**
                         * Constructs a new GetTransactionRequest.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a GetTransactionRequest.
                         * @implements IGetTransactionRequest
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.IGetTransactionRequest=} [properties] Properties to set
                         */
                        function GetTransactionRequest(properties) {
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * GetTransactionRequest id.
                         * @member {string} id
                         * @memberof org.dash.platform.dapi.v0.GetTransactionRequest
                         * @instance
                         */
                        GetTransactionRequest.prototype.id = "";

                        /**
                         * Creates a new GetTransactionRequest instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.GetTransactionRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetTransactionRequest=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.GetTransactionRequest} GetTransactionRequest instance
                         */
                        GetTransactionRequest.create = function create(properties) {
                            return new GetTransactionRequest(properties);
                        };

                        /**
                         * Encodes the specified GetTransactionRequest message. Does not implicitly {@link org.dash.platform.dapi.v0.GetTransactionRequest.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.GetTransactionRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetTransactionRequest} message GetTransactionRequest message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        GetTransactionRequest.encode = function encode(message, writer) {
                            if (!writer)
                                writer = $Writer.create();
                            if (message.id != null && Object.hasOwnProperty.call(message, "id"))
                                writer.uint32(/* id 1, wireType 2 =*/10).string(message.id);
                            return writer;
                        };

                        /**
                         * Encodes the specified GetTransactionRequest message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.GetTransactionRequest.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.GetTransactionRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetTransactionRequest} message GetTransactionRequest message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        GetTransactionRequest.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes a GetTransactionRequest message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.GetTransactionRequest
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.GetTransactionRequest} GetTransactionRequest
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        GetTransactionRequest.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.GetTransactionRequest();
                            while (reader.pos < end) {
                                var tag = reader.uint32();
                                switch (tag >>> 3) {
                                case 1:
                                    message.id = reader.string();
                                    break;
                                default:
                                    reader.skipType(tag & 7);
                                    break;
                                }
                            }
                            return message;
                        };

                        /**
                         * Decodes a GetTransactionRequest message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.GetTransactionRequest
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.GetTransactionRequest} GetTransactionRequest
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        GetTransactionRequest.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies a GetTransactionRequest message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.GetTransactionRequest
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        GetTransactionRequest.verify = function verify(message) {
                            if (typeof message !== "object" || message === null)
                                return "object expected";
                            if (message.id != null && message.hasOwnProperty("id"))
                                if (!$util.isString(message.id))
                                    return "id: string expected";
                            return null;
                        };

                        /**
                         * Creates a GetTransactionRequest message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.GetTransactionRequest
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.GetTransactionRequest} GetTransactionRequest
                         */
                        GetTransactionRequest.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.GetTransactionRequest)
                                return object;
                            var message = new $root.org.dash.platform.dapi.v0.GetTransactionRequest();
                            if (object.id != null)
                                message.id = String(object.id);
                            return message;
                        };

                        /**
                         * Creates a plain object from a GetTransactionRequest message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.GetTransactionRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.GetTransactionRequest} message GetTransactionRequest
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        GetTransactionRequest.toObject = function toObject(message, options) {
                            if (!options)
                                options = {};
                            var object = {};
                            if (options.defaults)
                                object.id = "";
                            if (message.id != null && message.hasOwnProperty("id"))
                                object.id = message.id;
                            return object;
                        };

                        /**
                         * Converts this GetTransactionRequest to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.GetTransactionRequest
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        GetTransactionRequest.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        return GetTransactionRequest;
                    })();

                    v0.GetTransactionResponse = (function() {

                        /**
                         * Properties of a GetTransactionResponse.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface IGetTransactionResponse
                         * @property {Uint8Array|null} [transaction] GetTransactionResponse transaction
                         * @property {Uint8Array|null} [blockHash] GetTransactionResponse blockHash
                         * @property {number|null} [height] GetTransactionResponse height
                         * @property {number|null} [confirmations] GetTransactionResponse confirmations
                         * @property {boolean|null} [isInstantLocked] GetTransactionResponse isInstantLocked
                         * @property {boolean|null} [isChainLocked] GetTransactionResponse isChainLocked
                         */

                        /**
                         * Constructs a new GetTransactionResponse.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a GetTransactionResponse.
                         * @implements IGetTransactionResponse
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.IGetTransactionResponse=} [properties] Properties to set
                         */
                        function GetTransactionResponse(properties) {
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * GetTransactionResponse transaction.
                         * @member {Uint8Array} transaction
                         * @memberof org.dash.platform.dapi.v0.GetTransactionResponse
                         * @instance
                         */
                        GetTransactionResponse.prototype.transaction = $util.newBuffer([]);

                        /**
                         * GetTransactionResponse blockHash.
                         * @member {Uint8Array} blockHash
                         * @memberof org.dash.platform.dapi.v0.GetTransactionResponse
                         * @instance
                         */
                        GetTransactionResponse.prototype.blockHash = $util.newBuffer([]);

                        /**
                         * GetTransactionResponse height.
                         * @member {number} height
                         * @memberof org.dash.platform.dapi.v0.GetTransactionResponse
                         * @instance
                         */
                        GetTransactionResponse.prototype.height = 0;

                        /**
                         * GetTransactionResponse confirmations.
                         * @member {number} confirmations
                         * @memberof org.dash.platform.dapi.v0.GetTransactionResponse
                         * @instance
                         */
                        GetTransactionResponse.prototype.confirmations = 0;

                        /**
                         * GetTransactionResponse isInstantLocked.
                         * @member {boolean} isInstantLocked
                         * @memberof org.dash.platform.dapi.v0.GetTransactionResponse
                         * @instance
                         */
                        GetTransactionResponse.prototype.isInstantLocked = false;

                        /**
                         * GetTransactionResponse isChainLocked.
                         * @member {boolean} isChainLocked
                         * @memberof org.dash.platform.dapi.v0.GetTransactionResponse
                         * @instance
                         */
                        GetTransactionResponse.prototype.isChainLocked = false;

                        /**
                         * Creates a new GetTransactionResponse instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.GetTransactionResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetTransactionResponse=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.GetTransactionResponse} GetTransactionResponse instance
                         */
                        GetTransactionResponse.create = function create(properties) {
                            return new GetTransactionResponse(properties);
                        };

                        /**
                         * Encodes the specified GetTransactionResponse message. Does not implicitly {@link org.dash.platform.dapi.v0.GetTransactionResponse.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.GetTransactionResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetTransactionResponse} message GetTransactionResponse message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        GetTransactionResponse.encode = function encode(message, writer) {
                            if (!writer)
                                writer = $Writer.create();
                            if (message.transaction != null && Object.hasOwnProperty.call(message, "transaction"))
                                writer.uint32(/* id 1, wireType 2 =*/10).bytes(message.transaction);
                            if (message.blockHash != null && Object.hasOwnProperty.call(message, "blockHash"))
                                writer.uint32(/* id 2, wireType 2 =*/18).bytes(message.blockHash);
                            if (message.height != null && Object.hasOwnProperty.call(message, "height"))
                                writer.uint32(/* id 3, wireType 0 =*/24).uint32(message.height);
                            if (message.confirmations != null && Object.hasOwnProperty.call(message, "confirmations"))
                                writer.uint32(/* id 4, wireType 0 =*/32).uint32(message.confirmations);
                            if (message.isInstantLocked != null && Object.hasOwnProperty.call(message, "isInstantLocked"))
                                writer.uint32(/* id 5, wireType 0 =*/40).bool(message.isInstantLocked);
                            if (message.isChainLocked != null && Object.hasOwnProperty.call(message, "isChainLocked"))
                                writer.uint32(/* id 6, wireType 0 =*/48).bool(message.isChainLocked);
                            return writer;
                        };

                        /**
                         * Encodes the specified GetTransactionResponse message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.GetTransactionResponse.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.GetTransactionResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetTransactionResponse} message GetTransactionResponse message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        GetTransactionResponse.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes a GetTransactionResponse message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.GetTransactionResponse
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.GetTransactionResponse} GetTransactionResponse
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        GetTransactionResponse.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.GetTransactionResponse();
                            while (reader.pos < end) {
                                var tag = reader.uint32();
                                switch (tag >>> 3) {
                                case 1:
                                    message.transaction = reader.bytes();
                                    break;
                                case 2:
                                    message.blockHash = reader.bytes();
                                    break;
                                case 3:
                                    message.height = reader.uint32();
                                    break;
                                case 4:
                                    message.confirmations = reader.uint32();
                                    break;
                                case 5:
                                    message.isInstantLocked = reader.bool();
                                    break;
                                case 6:
                                    message.isChainLocked = reader.bool();
                                    break;
                                default:
                                    reader.skipType(tag & 7);
                                    break;
                                }
                            }
                            return message;
                        };

                        /**
                         * Decodes a GetTransactionResponse message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.GetTransactionResponse
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.GetTransactionResponse} GetTransactionResponse
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        GetTransactionResponse.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies a GetTransactionResponse message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.GetTransactionResponse
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        GetTransactionResponse.verify = function verify(message) {
                            if (typeof message !== "object" || message === null)
                                return "object expected";
                            if (message.transaction != null && message.hasOwnProperty("transaction"))
                                if (!(message.transaction && typeof message.transaction.length === "number" || $util.isString(message.transaction)))
                                    return "transaction: buffer expected";
                            if (message.blockHash != null && message.hasOwnProperty("blockHash"))
                                if (!(message.blockHash && typeof message.blockHash.length === "number" || $util.isString(message.blockHash)))
                                    return "blockHash: buffer expected";
                            if (message.height != null && message.hasOwnProperty("height"))
                                if (!$util.isInteger(message.height))
                                    return "height: integer expected";
                            if (message.confirmations != null && message.hasOwnProperty("confirmations"))
                                if (!$util.isInteger(message.confirmations))
                                    return "confirmations: integer expected";
                            if (message.isInstantLocked != null && message.hasOwnProperty("isInstantLocked"))
                                if (typeof message.isInstantLocked !== "boolean")
                                    return "isInstantLocked: boolean expected";
                            if (message.isChainLocked != null && message.hasOwnProperty("isChainLocked"))
                                if (typeof message.isChainLocked !== "boolean")
                                    return "isChainLocked: boolean expected";
                            return null;
                        };

                        /**
                         * Creates a GetTransactionResponse message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.GetTransactionResponse
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.GetTransactionResponse} GetTransactionResponse
                         */
                        GetTransactionResponse.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.GetTransactionResponse)
                                return object;
                            var message = new $root.org.dash.platform.dapi.v0.GetTransactionResponse();
                            if (object.transaction != null)
                                if (typeof object.transaction === "string")
                                    $util.base64.decode(object.transaction, message.transaction = $util.newBuffer($util.base64.length(object.transaction)), 0);
                                else if (object.transaction.length >= 0)
                                    message.transaction = object.transaction;
                            if (object.blockHash != null)
                                if (typeof object.blockHash === "string")
                                    $util.base64.decode(object.blockHash, message.blockHash = $util.newBuffer($util.base64.length(object.blockHash)), 0);
                                else if (object.blockHash.length >= 0)
                                    message.blockHash = object.blockHash;
                            if (object.height != null)
                                message.height = object.height >>> 0;
                            if (object.confirmations != null)
                                message.confirmations = object.confirmations >>> 0;
                            if (object.isInstantLocked != null)
                                message.isInstantLocked = Boolean(object.isInstantLocked);
                            if (object.isChainLocked != null)
                                message.isChainLocked = Boolean(object.isChainLocked);
                            return message;
                        };

                        /**
                         * Creates a plain object from a GetTransactionResponse message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.GetTransactionResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.GetTransactionResponse} message GetTransactionResponse
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        GetTransactionResponse.toObject = function toObject(message, options) {
                            if (!options)
                                options = {};
                            var object = {};
                            if (options.defaults) {
                                if (options.bytes === String)
                                    object.transaction = "";
                                else {
                                    object.transaction = [];
                                    if (options.bytes !== Array)
                                        object.transaction = $util.newBuffer(object.transaction);
                                }
                                if (options.bytes === String)
                                    object.blockHash = "";
                                else {
                                    object.blockHash = [];
                                    if (options.bytes !== Array)
                                        object.blockHash = $util.newBuffer(object.blockHash);
                                }
                                object.height = 0;
                                object.confirmations = 0;
                                object.isInstantLocked = false;
                                object.isChainLocked = false;
                            }
                            if (message.transaction != null && message.hasOwnProperty("transaction"))
                                object.transaction = options.bytes === String ? $util.base64.encode(message.transaction, 0, message.transaction.length) : options.bytes === Array ? Array.prototype.slice.call(message.transaction) : message.transaction;
                            if (message.blockHash != null && message.hasOwnProperty("blockHash"))
                                object.blockHash = options.bytes === String ? $util.base64.encode(message.blockHash, 0, message.blockHash.length) : options.bytes === Array ? Array.prototype.slice.call(message.blockHash) : message.blockHash;
                            if (message.height != null && message.hasOwnProperty("height"))
                                object.height = message.height;
                            if (message.confirmations != null && message.hasOwnProperty("confirmations"))
                                object.confirmations = message.confirmations;
                            if (message.isInstantLocked != null && message.hasOwnProperty("isInstantLocked"))
                                object.isInstantLocked = message.isInstantLocked;
                            if (message.isChainLocked != null && message.hasOwnProperty("isChainLocked"))
                                object.isChainLocked = message.isChainLocked;
                            return object;
                        };

                        /**
                         * Converts this GetTransactionResponse to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.GetTransactionResponse
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        GetTransactionResponse.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        return GetTransactionResponse;
                    })();

                    v0.BlockHeadersWithChainLocksRequest = (function() {

                        /**
                         * Properties of a BlockHeadersWithChainLocksRequest.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface IBlockHeadersWithChainLocksRequest
                         * @property {Uint8Array|null} [fromBlockHash] BlockHeadersWithChainLocksRequest fromBlockHash
                         * @property {number|null} [fromBlockHeight] BlockHeadersWithChainLocksRequest fromBlockHeight
                         * @property {number|null} [count] BlockHeadersWithChainLocksRequest count
                         */

                        /**
                         * Constructs a new BlockHeadersWithChainLocksRequest.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a BlockHeadersWithChainLocksRequest.
                         * @implements IBlockHeadersWithChainLocksRequest
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.IBlockHeadersWithChainLocksRequest=} [properties] Properties to set
                         */
                        function BlockHeadersWithChainLocksRequest(properties) {
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * BlockHeadersWithChainLocksRequest fromBlockHash.
                         * @member {Uint8Array} fromBlockHash
                         * @memberof org.dash.platform.dapi.v0.BlockHeadersWithChainLocksRequest
                         * @instance
                         */
                        BlockHeadersWithChainLocksRequest.prototype.fromBlockHash = $util.newBuffer([]);

                        /**
                         * BlockHeadersWithChainLocksRequest fromBlockHeight.
                         * @member {number} fromBlockHeight
                         * @memberof org.dash.platform.dapi.v0.BlockHeadersWithChainLocksRequest
                         * @instance
                         */
                        BlockHeadersWithChainLocksRequest.prototype.fromBlockHeight = 0;

                        /**
                         * BlockHeadersWithChainLocksRequest count.
                         * @member {number} count
                         * @memberof org.dash.platform.dapi.v0.BlockHeadersWithChainLocksRequest
                         * @instance
                         */
                        BlockHeadersWithChainLocksRequest.prototype.count = 0;

                        // OneOf field names bound to virtual getters and setters
                        var $oneOfFields;

                        /**
                         * BlockHeadersWithChainLocksRequest fromBlock.
                         * @member {"fromBlockHash"|"fromBlockHeight"|undefined} fromBlock
                         * @memberof org.dash.platform.dapi.v0.BlockHeadersWithChainLocksRequest
                         * @instance
                         */
                        Object.defineProperty(BlockHeadersWithChainLocksRequest.prototype, "fromBlock", {
                            get: $util.oneOfGetter($oneOfFields = ["fromBlockHash", "fromBlockHeight"]),
                            set: $util.oneOfSetter($oneOfFields)
                        });

                        /**
                         * Creates a new BlockHeadersWithChainLocksRequest instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.BlockHeadersWithChainLocksRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.IBlockHeadersWithChainLocksRequest=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.BlockHeadersWithChainLocksRequest} BlockHeadersWithChainLocksRequest instance
                         */
                        BlockHeadersWithChainLocksRequest.create = function create(properties) {
                            return new BlockHeadersWithChainLocksRequest(properties);
                        };

                        /**
                         * Encodes the specified BlockHeadersWithChainLocksRequest message. Does not implicitly {@link org.dash.platform.dapi.v0.BlockHeadersWithChainLocksRequest.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.BlockHeadersWithChainLocksRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.IBlockHeadersWithChainLocksRequest} message BlockHeadersWithChainLocksRequest message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        BlockHeadersWithChainLocksRequest.encode = function encode(message, writer) {
                            if (!writer)
                                writer = $Writer.create();
                            if (message.fromBlockHash != null && Object.hasOwnProperty.call(message, "fromBlockHash"))
                                writer.uint32(/* id 1, wireType 2 =*/10).bytes(message.fromBlockHash);
                            if (message.fromBlockHeight != null && Object.hasOwnProperty.call(message, "fromBlockHeight"))
                                writer.uint32(/* id 2, wireType 0 =*/16).uint32(message.fromBlockHeight);
                            if (message.count != null && Object.hasOwnProperty.call(message, "count"))
                                writer.uint32(/* id 3, wireType 0 =*/24).uint32(message.count);
                            return writer;
                        };

                        /**
                         * Encodes the specified BlockHeadersWithChainLocksRequest message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.BlockHeadersWithChainLocksRequest.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.BlockHeadersWithChainLocksRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.IBlockHeadersWithChainLocksRequest} message BlockHeadersWithChainLocksRequest message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        BlockHeadersWithChainLocksRequest.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes a BlockHeadersWithChainLocksRequest message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.BlockHeadersWithChainLocksRequest
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.BlockHeadersWithChainLocksRequest} BlockHeadersWithChainLocksRequest
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        BlockHeadersWithChainLocksRequest.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.BlockHeadersWithChainLocksRequest();
                            while (reader.pos < end) {
                                var tag = reader.uint32();
                                switch (tag >>> 3) {
                                case 1:
                                    message.fromBlockHash = reader.bytes();
                                    break;
                                case 2:
                                    message.fromBlockHeight = reader.uint32();
                                    break;
                                case 3:
                                    message.count = reader.uint32();
                                    break;
                                default:
                                    reader.skipType(tag & 7);
                                    break;
                                }
                            }
                            return message;
                        };

                        /**
                         * Decodes a BlockHeadersWithChainLocksRequest message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.BlockHeadersWithChainLocksRequest
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.BlockHeadersWithChainLocksRequest} BlockHeadersWithChainLocksRequest
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        BlockHeadersWithChainLocksRequest.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies a BlockHeadersWithChainLocksRequest message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.BlockHeadersWithChainLocksRequest
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        BlockHeadersWithChainLocksRequest.verify = function verify(message) {
                            if (typeof message !== "object" || message === null)
                                return "object expected";
                            var properties = {};
                            if (message.fromBlockHash != null && message.hasOwnProperty("fromBlockHash")) {
                                properties.fromBlock = 1;
                                if (!(message.fromBlockHash && typeof message.fromBlockHash.length === "number" || $util.isString(message.fromBlockHash)))
                                    return "fromBlockHash: buffer expected";
                            }
                            if (message.fromBlockHeight != null && message.hasOwnProperty("fromBlockHeight")) {
                                if (properties.fromBlock === 1)
                                    return "fromBlock: multiple values";
                                properties.fromBlock = 1;
                                if (!$util.isInteger(message.fromBlockHeight))
                                    return "fromBlockHeight: integer expected";
                            }
                            if (message.count != null && message.hasOwnProperty("count"))
                                if (!$util.isInteger(message.count))
                                    return "count: integer expected";
                            return null;
                        };

                        /**
                         * Creates a BlockHeadersWithChainLocksRequest message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.BlockHeadersWithChainLocksRequest
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.BlockHeadersWithChainLocksRequest} BlockHeadersWithChainLocksRequest
                         */
                        BlockHeadersWithChainLocksRequest.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.BlockHeadersWithChainLocksRequest)
                                return object;
                            var message = new $root.org.dash.platform.dapi.v0.BlockHeadersWithChainLocksRequest();
                            if (object.fromBlockHash != null)
                                if (typeof object.fromBlockHash === "string")
                                    $util.base64.decode(object.fromBlockHash, message.fromBlockHash = $util.newBuffer($util.base64.length(object.fromBlockHash)), 0);
                                else if (object.fromBlockHash.length >= 0)
                                    message.fromBlockHash = object.fromBlockHash;
                            if (object.fromBlockHeight != null)
                                message.fromBlockHeight = object.fromBlockHeight >>> 0;
                            if (object.count != null)
                                message.count = object.count >>> 0;
                            return message;
                        };

                        /**
                         * Creates a plain object from a BlockHeadersWithChainLocksRequest message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.BlockHeadersWithChainLocksRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.BlockHeadersWithChainLocksRequest} message BlockHeadersWithChainLocksRequest
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        BlockHeadersWithChainLocksRequest.toObject = function toObject(message, options) {
                            if (!options)
                                options = {};
                            var object = {};
                            if (options.defaults)
                                object.count = 0;
                            if (message.fromBlockHash != null && message.hasOwnProperty("fromBlockHash")) {
                                object.fromBlockHash = options.bytes === String ? $util.base64.encode(message.fromBlockHash, 0, message.fromBlockHash.length) : options.bytes === Array ? Array.prototype.slice.call(message.fromBlockHash) : message.fromBlockHash;
                                if (options.oneofs)
                                    object.fromBlock = "fromBlockHash";
                            }
                            if (message.fromBlockHeight != null && message.hasOwnProperty("fromBlockHeight")) {
                                object.fromBlockHeight = message.fromBlockHeight;
                                if (options.oneofs)
                                    object.fromBlock = "fromBlockHeight";
                            }
                            if (message.count != null && message.hasOwnProperty("count"))
                                object.count = message.count;
                            return object;
                        };

                        /**
                         * Converts this BlockHeadersWithChainLocksRequest to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.BlockHeadersWithChainLocksRequest
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        BlockHeadersWithChainLocksRequest.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        return BlockHeadersWithChainLocksRequest;
                    })();

                    v0.BlockHeadersWithChainLocksResponse = (function() {

                        /**
                         * Properties of a BlockHeadersWithChainLocksResponse.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface IBlockHeadersWithChainLocksResponse
                         * @property {org.dash.platform.dapi.v0.IBlockHeaders|null} [blockHeaders] BlockHeadersWithChainLocksResponse blockHeaders
                         * @property {org.dash.platform.dapi.v0.IChainLockSignatureMessages|null} [chainLockSignatureMessages] BlockHeadersWithChainLocksResponse chainLockSignatureMessages
                         */

                        /**
                         * Constructs a new BlockHeadersWithChainLocksResponse.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a BlockHeadersWithChainLocksResponse.
                         * @implements IBlockHeadersWithChainLocksResponse
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.IBlockHeadersWithChainLocksResponse=} [properties] Properties to set
                         */
                        function BlockHeadersWithChainLocksResponse(properties) {
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * BlockHeadersWithChainLocksResponse blockHeaders.
                         * @member {org.dash.platform.dapi.v0.IBlockHeaders|null|undefined} blockHeaders
                         * @memberof org.dash.platform.dapi.v0.BlockHeadersWithChainLocksResponse
                         * @instance
                         */
                        BlockHeadersWithChainLocksResponse.prototype.blockHeaders = null;

                        /**
                         * BlockHeadersWithChainLocksResponse chainLockSignatureMessages.
                         * @member {org.dash.platform.dapi.v0.IChainLockSignatureMessages|null|undefined} chainLockSignatureMessages
                         * @memberof org.dash.platform.dapi.v0.BlockHeadersWithChainLocksResponse
                         * @instance
                         */
                        BlockHeadersWithChainLocksResponse.prototype.chainLockSignatureMessages = null;

                        // OneOf field names bound to virtual getters and setters
                        var $oneOfFields;

                        /**
                         * BlockHeadersWithChainLocksResponse responses.
                         * @member {"blockHeaders"|"chainLockSignatureMessages"|undefined} responses
                         * @memberof org.dash.platform.dapi.v0.BlockHeadersWithChainLocksResponse
                         * @instance
                         */
                        Object.defineProperty(BlockHeadersWithChainLocksResponse.prototype, "responses", {
                            get: $util.oneOfGetter($oneOfFields = ["blockHeaders", "chainLockSignatureMessages"]),
                            set: $util.oneOfSetter($oneOfFields)
                        });

                        /**
                         * Creates a new BlockHeadersWithChainLocksResponse instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.BlockHeadersWithChainLocksResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.IBlockHeadersWithChainLocksResponse=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.BlockHeadersWithChainLocksResponse} BlockHeadersWithChainLocksResponse instance
                         */
                        BlockHeadersWithChainLocksResponse.create = function create(properties) {
                            return new BlockHeadersWithChainLocksResponse(properties);
                        };

                        /**
                         * Encodes the specified BlockHeadersWithChainLocksResponse message. Does not implicitly {@link org.dash.platform.dapi.v0.BlockHeadersWithChainLocksResponse.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.BlockHeadersWithChainLocksResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.IBlockHeadersWithChainLocksResponse} message BlockHeadersWithChainLocksResponse message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        BlockHeadersWithChainLocksResponse.encode = function encode(message, writer) {
                            if (!writer)
                                writer = $Writer.create();
                            if (message.blockHeaders != null && Object.hasOwnProperty.call(message, "blockHeaders"))
                                $root.org.dash.platform.dapi.v0.BlockHeaders.encode(message.blockHeaders, writer.uint32(/* id 1, wireType 2 =*/10).fork()).ldelim();
                            if (message.chainLockSignatureMessages != null && Object.hasOwnProperty.call(message, "chainLockSignatureMessages"))
                                $root.org.dash.platform.dapi.v0.ChainLockSignatureMessages.encode(message.chainLockSignatureMessages, writer.uint32(/* id 2, wireType 2 =*/18).fork()).ldelim();
                            return writer;
                        };

                        /**
                         * Encodes the specified BlockHeadersWithChainLocksResponse message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.BlockHeadersWithChainLocksResponse.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.BlockHeadersWithChainLocksResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.IBlockHeadersWithChainLocksResponse} message BlockHeadersWithChainLocksResponse message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        BlockHeadersWithChainLocksResponse.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes a BlockHeadersWithChainLocksResponse message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.BlockHeadersWithChainLocksResponse
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.BlockHeadersWithChainLocksResponse} BlockHeadersWithChainLocksResponse
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        BlockHeadersWithChainLocksResponse.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.BlockHeadersWithChainLocksResponse();
                            while (reader.pos < end) {
                                var tag = reader.uint32();
                                switch (tag >>> 3) {
                                case 1:
                                    message.blockHeaders = $root.org.dash.platform.dapi.v0.BlockHeaders.decode(reader, reader.uint32());
                                    break;
                                case 2:
                                    message.chainLockSignatureMessages = $root.org.dash.platform.dapi.v0.ChainLockSignatureMessages.decode(reader, reader.uint32());
                                    break;
                                default:
                                    reader.skipType(tag & 7);
                                    break;
                                }
                            }
                            return message;
                        };

                        /**
                         * Decodes a BlockHeadersWithChainLocksResponse message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.BlockHeadersWithChainLocksResponse
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.BlockHeadersWithChainLocksResponse} BlockHeadersWithChainLocksResponse
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        BlockHeadersWithChainLocksResponse.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies a BlockHeadersWithChainLocksResponse message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.BlockHeadersWithChainLocksResponse
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        BlockHeadersWithChainLocksResponse.verify = function verify(message) {
                            if (typeof message !== "object" || message === null)
                                return "object expected";
                            var properties = {};
                            if (message.blockHeaders != null && message.hasOwnProperty("blockHeaders")) {
                                properties.responses = 1;
                                {
                                    var error = $root.org.dash.platform.dapi.v0.BlockHeaders.verify(message.blockHeaders);
                                    if (error)
                                        return "blockHeaders." + error;
                                }
                            }
                            if (message.chainLockSignatureMessages != null && message.hasOwnProperty("chainLockSignatureMessages")) {
                                if (properties.responses === 1)
                                    return "responses: multiple values";
                                properties.responses = 1;
                                {
                                    var error = $root.org.dash.platform.dapi.v0.ChainLockSignatureMessages.verify(message.chainLockSignatureMessages);
                                    if (error)
                                        return "chainLockSignatureMessages." + error;
                                }
                            }
                            return null;
                        };

                        /**
                         * Creates a BlockHeadersWithChainLocksResponse message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.BlockHeadersWithChainLocksResponse
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.BlockHeadersWithChainLocksResponse} BlockHeadersWithChainLocksResponse
                         */
                        BlockHeadersWithChainLocksResponse.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.BlockHeadersWithChainLocksResponse)
                                return object;
                            var message = new $root.org.dash.platform.dapi.v0.BlockHeadersWithChainLocksResponse();
                            if (object.blockHeaders != null) {
                                if (typeof object.blockHeaders !== "object")
                                    throw TypeError(".org.dash.platform.dapi.v0.BlockHeadersWithChainLocksResponse.blockHeaders: object expected");
                                message.blockHeaders = $root.org.dash.platform.dapi.v0.BlockHeaders.fromObject(object.blockHeaders);
                            }
                            if (object.chainLockSignatureMessages != null) {
                                if (typeof object.chainLockSignatureMessages !== "object")
                                    throw TypeError(".org.dash.platform.dapi.v0.BlockHeadersWithChainLocksResponse.chainLockSignatureMessages: object expected");
                                message.chainLockSignatureMessages = $root.org.dash.platform.dapi.v0.ChainLockSignatureMessages.fromObject(object.chainLockSignatureMessages);
                            }
                            return message;
                        };

                        /**
                         * Creates a plain object from a BlockHeadersWithChainLocksResponse message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.BlockHeadersWithChainLocksResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.BlockHeadersWithChainLocksResponse} message BlockHeadersWithChainLocksResponse
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        BlockHeadersWithChainLocksResponse.toObject = function toObject(message, options) {
                            if (!options)
                                options = {};
                            var object = {};
                            if (message.blockHeaders != null && message.hasOwnProperty("blockHeaders")) {
                                object.blockHeaders = $root.org.dash.platform.dapi.v0.BlockHeaders.toObject(message.blockHeaders, options);
                                if (options.oneofs)
                                    object.responses = "blockHeaders";
                            }
                            if (message.chainLockSignatureMessages != null && message.hasOwnProperty("chainLockSignatureMessages")) {
                                object.chainLockSignatureMessages = $root.org.dash.platform.dapi.v0.ChainLockSignatureMessages.toObject(message.chainLockSignatureMessages, options);
                                if (options.oneofs)
                                    object.responses = "chainLockSignatureMessages";
                            }
                            return object;
                        };

                        /**
                         * Converts this BlockHeadersWithChainLocksResponse to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.BlockHeadersWithChainLocksResponse
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        BlockHeadersWithChainLocksResponse.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        return BlockHeadersWithChainLocksResponse;
                    })();

                    v0.BlockHeaders = (function() {

                        /**
                         * Properties of a BlockHeaders.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface IBlockHeaders
                         * @property {Array.<Uint8Array>|null} [headers] BlockHeaders headers
                         */

                        /**
                         * Constructs a new BlockHeaders.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a BlockHeaders.
                         * @implements IBlockHeaders
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.IBlockHeaders=} [properties] Properties to set
                         */
                        function BlockHeaders(properties) {
                            this.headers = [];
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * BlockHeaders headers.
                         * @member {Array.<Uint8Array>} headers
                         * @memberof org.dash.platform.dapi.v0.BlockHeaders
                         * @instance
                         */
                        BlockHeaders.prototype.headers = $util.emptyArray;

                        /**
                         * Creates a new BlockHeaders instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.BlockHeaders
                         * @static
                         * @param {org.dash.platform.dapi.v0.IBlockHeaders=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.BlockHeaders} BlockHeaders instance
                         */
                        BlockHeaders.create = function create(properties) {
                            return new BlockHeaders(properties);
                        };

                        /**
                         * Encodes the specified BlockHeaders message. Does not implicitly {@link org.dash.platform.dapi.v0.BlockHeaders.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.BlockHeaders
                         * @static
                         * @param {org.dash.platform.dapi.v0.IBlockHeaders} message BlockHeaders message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        BlockHeaders.encode = function encode(message, writer) {
                            if (!writer)
                                writer = $Writer.create();
                            if (message.headers != null && message.headers.length)
                                for (var i = 0; i < message.headers.length; ++i)
                                    writer.uint32(/* id 1, wireType 2 =*/10).bytes(message.headers[i]);
                            return writer;
                        };

                        /**
                         * Encodes the specified BlockHeaders message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.BlockHeaders.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.BlockHeaders
                         * @static
                         * @param {org.dash.platform.dapi.v0.IBlockHeaders} message BlockHeaders message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        BlockHeaders.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes a BlockHeaders message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.BlockHeaders
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.BlockHeaders} BlockHeaders
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        BlockHeaders.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.BlockHeaders();
                            while (reader.pos < end) {
                                var tag = reader.uint32();
                                switch (tag >>> 3) {
                                case 1:
                                    if (!(message.headers && message.headers.length))
                                        message.headers = [];
                                    message.headers.push(reader.bytes());
                                    break;
                                default:
                                    reader.skipType(tag & 7);
                                    break;
                                }
                            }
                            return message;
                        };

                        /**
                         * Decodes a BlockHeaders message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.BlockHeaders
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.BlockHeaders} BlockHeaders
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        BlockHeaders.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies a BlockHeaders message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.BlockHeaders
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        BlockHeaders.verify = function verify(message) {
                            if (typeof message !== "object" || message === null)
                                return "object expected";
                            if (message.headers != null && message.hasOwnProperty("headers")) {
                                if (!Array.isArray(message.headers))
                                    return "headers: array expected";
                                for (var i = 0; i < message.headers.length; ++i)
                                    if (!(message.headers[i] && typeof message.headers[i].length === "number" || $util.isString(message.headers[i])))
                                        return "headers: buffer[] expected";
                            }
                            return null;
                        };

                        /**
                         * Creates a BlockHeaders message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.BlockHeaders
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.BlockHeaders} BlockHeaders
                         */
                        BlockHeaders.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.BlockHeaders)
                                return object;
                            var message = new $root.org.dash.platform.dapi.v0.BlockHeaders();
                            if (object.headers) {
                                if (!Array.isArray(object.headers))
                                    throw TypeError(".org.dash.platform.dapi.v0.BlockHeaders.headers: array expected");
                                message.headers = [];
                                for (var i = 0; i < object.headers.length; ++i)
                                    if (typeof object.headers[i] === "string")
                                        $util.base64.decode(object.headers[i], message.headers[i] = $util.newBuffer($util.base64.length(object.headers[i])), 0);
                                    else if (object.headers[i].length >= 0)
                                        message.headers[i] = object.headers[i];
                            }
                            return message;
                        };

                        /**
                         * Creates a plain object from a BlockHeaders message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.BlockHeaders
                         * @static
                         * @param {org.dash.platform.dapi.v0.BlockHeaders} message BlockHeaders
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        BlockHeaders.toObject = function toObject(message, options) {
                            if (!options)
                                options = {};
                            var object = {};
                            if (options.arrays || options.defaults)
                                object.headers = [];
                            if (message.headers && message.headers.length) {
                                object.headers = [];
                                for (var j = 0; j < message.headers.length; ++j)
                                    object.headers[j] = options.bytes === String ? $util.base64.encode(message.headers[j], 0, message.headers[j].length) : options.bytes === Array ? Array.prototype.slice.call(message.headers[j]) : message.headers[j];
                            }
                            return object;
                        };

                        /**
                         * Converts this BlockHeaders to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.BlockHeaders
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        BlockHeaders.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        return BlockHeaders;
                    })();

                    v0.ChainLockSignatureMessages = (function() {

                        /**
                         * Properties of a ChainLockSignatureMessages.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface IChainLockSignatureMessages
                         * @property {Array.<Uint8Array>|null} [messages] ChainLockSignatureMessages messages
                         */

                        /**
                         * Constructs a new ChainLockSignatureMessages.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a ChainLockSignatureMessages.
                         * @implements IChainLockSignatureMessages
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.IChainLockSignatureMessages=} [properties] Properties to set
                         */
                        function ChainLockSignatureMessages(properties) {
                            this.messages = [];
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * ChainLockSignatureMessages messages.
                         * @member {Array.<Uint8Array>} messages
                         * @memberof org.dash.platform.dapi.v0.ChainLockSignatureMessages
                         * @instance
                         */
                        ChainLockSignatureMessages.prototype.messages = $util.emptyArray;

                        /**
                         * Creates a new ChainLockSignatureMessages instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.ChainLockSignatureMessages
                         * @static
                         * @param {org.dash.platform.dapi.v0.IChainLockSignatureMessages=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.ChainLockSignatureMessages} ChainLockSignatureMessages instance
                         */
                        ChainLockSignatureMessages.create = function create(properties) {
                            return new ChainLockSignatureMessages(properties);
                        };

                        /**
                         * Encodes the specified ChainLockSignatureMessages message. Does not implicitly {@link org.dash.platform.dapi.v0.ChainLockSignatureMessages.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.ChainLockSignatureMessages
                         * @static
                         * @param {org.dash.platform.dapi.v0.IChainLockSignatureMessages} message ChainLockSignatureMessages message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        ChainLockSignatureMessages.encode = function encode(message, writer) {
                            if (!writer)
                                writer = $Writer.create();
                            if (message.messages != null && message.messages.length)
                                for (var i = 0; i < message.messages.length; ++i)
                                    writer.uint32(/* id 1, wireType 2 =*/10).bytes(message.messages[i]);
                            return writer;
                        };

                        /**
                         * Encodes the specified ChainLockSignatureMessages message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.ChainLockSignatureMessages.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.ChainLockSignatureMessages
                         * @static
                         * @param {org.dash.platform.dapi.v0.IChainLockSignatureMessages} message ChainLockSignatureMessages message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        ChainLockSignatureMessages.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes a ChainLockSignatureMessages message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.ChainLockSignatureMessages
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.ChainLockSignatureMessages} ChainLockSignatureMessages
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        ChainLockSignatureMessages.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.ChainLockSignatureMessages();
                            while (reader.pos < end) {
                                var tag = reader.uint32();
                                switch (tag >>> 3) {
                                case 1:
                                    if (!(message.messages && message.messages.length))
                                        message.messages = [];
                                    message.messages.push(reader.bytes());
                                    break;
                                default:
                                    reader.skipType(tag & 7);
                                    break;
                                }
                            }
                            return message;
                        };

                        /**
                         * Decodes a ChainLockSignatureMessages message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.ChainLockSignatureMessages
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.ChainLockSignatureMessages} ChainLockSignatureMessages
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        ChainLockSignatureMessages.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies a ChainLockSignatureMessages message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.ChainLockSignatureMessages
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        ChainLockSignatureMessages.verify = function verify(message) {
                            if (typeof message !== "object" || message === null)
                                return "object expected";
                            if (message.messages != null && message.hasOwnProperty("messages")) {
                                if (!Array.isArray(message.messages))
                                    return "messages: array expected";
                                for (var i = 0; i < message.messages.length; ++i)
                                    if (!(message.messages[i] && typeof message.messages[i].length === "number" || $util.isString(message.messages[i])))
                                        return "messages: buffer[] expected";
                            }
                            return null;
                        };

                        /**
                         * Creates a ChainLockSignatureMessages message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.ChainLockSignatureMessages
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.ChainLockSignatureMessages} ChainLockSignatureMessages
                         */
                        ChainLockSignatureMessages.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.ChainLockSignatureMessages)
                                return object;
                            var message = new $root.org.dash.platform.dapi.v0.ChainLockSignatureMessages();
                            if (object.messages) {
                                if (!Array.isArray(object.messages))
                                    throw TypeError(".org.dash.platform.dapi.v0.ChainLockSignatureMessages.messages: array expected");
                                message.messages = [];
                                for (var i = 0; i < object.messages.length; ++i)
                                    if (typeof object.messages[i] === "string")
                                        $util.base64.decode(object.messages[i], message.messages[i] = $util.newBuffer($util.base64.length(object.messages[i])), 0);
                                    else if (object.messages[i].length >= 0)
                                        message.messages[i] = object.messages[i];
                            }
                            return message;
                        };

                        /**
                         * Creates a plain object from a ChainLockSignatureMessages message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.ChainLockSignatureMessages
                         * @static
                         * @param {org.dash.platform.dapi.v0.ChainLockSignatureMessages} message ChainLockSignatureMessages
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        ChainLockSignatureMessages.toObject = function toObject(message, options) {
                            if (!options)
                                options = {};
                            var object = {};
                            if (options.arrays || options.defaults)
                                object.messages = [];
                            if (message.messages && message.messages.length) {
                                object.messages = [];
                                for (var j = 0; j < message.messages.length; ++j)
                                    object.messages[j] = options.bytes === String ? $util.base64.encode(message.messages[j], 0, message.messages[j].length) : options.bytes === Array ? Array.prototype.slice.call(message.messages[j]) : message.messages[j];
                            }
                            return object;
                        };

                        /**
                         * Converts this ChainLockSignatureMessages to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.ChainLockSignatureMessages
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        ChainLockSignatureMessages.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        return ChainLockSignatureMessages;
                    })();

                    v0.GetEstimatedTransactionFeeRequest = (function() {

                        /**
                         * Properties of a GetEstimatedTransactionFeeRequest.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface IGetEstimatedTransactionFeeRequest
                         * @property {number|null} [blocks] GetEstimatedTransactionFeeRequest blocks
                         */

                        /**
                         * Constructs a new GetEstimatedTransactionFeeRequest.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a GetEstimatedTransactionFeeRequest.
                         * @implements IGetEstimatedTransactionFeeRequest
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.IGetEstimatedTransactionFeeRequest=} [properties] Properties to set
                         */
                        function GetEstimatedTransactionFeeRequest(properties) {
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * GetEstimatedTransactionFeeRequest blocks.
                         * @member {number} blocks
                         * @memberof org.dash.platform.dapi.v0.GetEstimatedTransactionFeeRequest
                         * @instance
                         */
                        GetEstimatedTransactionFeeRequest.prototype.blocks = 0;

                        /**
                         * Creates a new GetEstimatedTransactionFeeRequest instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.GetEstimatedTransactionFeeRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetEstimatedTransactionFeeRequest=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.GetEstimatedTransactionFeeRequest} GetEstimatedTransactionFeeRequest instance
                         */
                        GetEstimatedTransactionFeeRequest.create = function create(properties) {
                            return new GetEstimatedTransactionFeeRequest(properties);
                        };

                        /**
                         * Encodes the specified GetEstimatedTransactionFeeRequest message. Does not implicitly {@link org.dash.platform.dapi.v0.GetEstimatedTransactionFeeRequest.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.GetEstimatedTransactionFeeRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetEstimatedTransactionFeeRequest} message GetEstimatedTransactionFeeRequest message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        GetEstimatedTransactionFeeRequest.encode = function encode(message, writer) {
                            if (!writer)
                                writer = $Writer.create();
                            if (message.blocks != null && Object.hasOwnProperty.call(message, "blocks"))
                                writer.uint32(/* id 1, wireType 0 =*/8).uint32(message.blocks);
                            return writer;
                        };

                        /**
                         * Encodes the specified GetEstimatedTransactionFeeRequest message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.GetEstimatedTransactionFeeRequest.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.GetEstimatedTransactionFeeRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetEstimatedTransactionFeeRequest} message GetEstimatedTransactionFeeRequest message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        GetEstimatedTransactionFeeRequest.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes a GetEstimatedTransactionFeeRequest message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.GetEstimatedTransactionFeeRequest
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.GetEstimatedTransactionFeeRequest} GetEstimatedTransactionFeeRequest
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        GetEstimatedTransactionFeeRequest.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.GetEstimatedTransactionFeeRequest();
                            while (reader.pos < end) {
                                var tag = reader.uint32();
                                switch (tag >>> 3) {
                                case 1:
                                    message.blocks = reader.uint32();
                                    break;
                                default:
                                    reader.skipType(tag & 7);
                                    break;
                                }
                            }
                            return message;
                        };

                        /**
                         * Decodes a GetEstimatedTransactionFeeRequest message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.GetEstimatedTransactionFeeRequest
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.GetEstimatedTransactionFeeRequest} GetEstimatedTransactionFeeRequest
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        GetEstimatedTransactionFeeRequest.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies a GetEstimatedTransactionFeeRequest message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.GetEstimatedTransactionFeeRequest
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        GetEstimatedTransactionFeeRequest.verify = function verify(message) {
                            if (typeof message !== "object" || message === null)
                                return "object expected";
                            if (message.blocks != null && message.hasOwnProperty("blocks"))
                                if (!$util.isInteger(message.blocks))
                                    return "blocks: integer expected";
                            return null;
                        };

                        /**
                         * Creates a GetEstimatedTransactionFeeRequest message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.GetEstimatedTransactionFeeRequest
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.GetEstimatedTransactionFeeRequest} GetEstimatedTransactionFeeRequest
                         */
                        GetEstimatedTransactionFeeRequest.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.GetEstimatedTransactionFeeRequest)
                                return object;
                            var message = new $root.org.dash.platform.dapi.v0.GetEstimatedTransactionFeeRequest();
                            if (object.blocks != null)
                                message.blocks = object.blocks >>> 0;
                            return message;
                        };

                        /**
                         * Creates a plain object from a GetEstimatedTransactionFeeRequest message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.GetEstimatedTransactionFeeRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.GetEstimatedTransactionFeeRequest} message GetEstimatedTransactionFeeRequest
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        GetEstimatedTransactionFeeRequest.toObject = function toObject(message, options) {
                            if (!options)
                                options = {};
                            var object = {};
                            if (options.defaults)
                                object.blocks = 0;
                            if (message.blocks != null && message.hasOwnProperty("blocks"))
                                object.blocks = message.blocks;
                            return object;
                        };

                        /**
                         * Converts this GetEstimatedTransactionFeeRequest to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.GetEstimatedTransactionFeeRequest
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        GetEstimatedTransactionFeeRequest.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        return GetEstimatedTransactionFeeRequest;
                    })();

                    v0.GetEstimatedTransactionFeeResponse = (function() {

                        /**
                         * Properties of a GetEstimatedTransactionFeeResponse.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface IGetEstimatedTransactionFeeResponse
                         * @property {number|null} [fee] GetEstimatedTransactionFeeResponse fee
                         */

                        /**
                         * Constructs a new GetEstimatedTransactionFeeResponse.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a GetEstimatedTransactionFeeResponse.
                         * @implements IGetEstimatedTransactionFeeResponse
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.IGetEstimatedTransactionFeeResponse=} [properties] Properties to set
                         */
                        function GetEstimatedTransactionFeeResponse(properties) {
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * GetEstimatedTransactionFeeResponse fee.
                         * @member {number} fee
                         * @memberof org.dash.platform.dapi.v0.GetEstimatedTransactionFeeResponse
                         * @instance
                         */
                        GetEstimatedTransactionFeeResponse.prototype.fee = 0;

                        /**
                         * Creates a new GetEstimatedTransactionFeeResponse instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.GetEstimatedTransactionFeeResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetEstimatedTransactionFeeResponse=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.GetEstimatedTransactionFeeResponse} GetEstimatedTransactionFeeResponse instance
                         */
                        GetEstimatedTransactionFeeResponse.create = function create(properties) {
                            return new GetEstimatedTransactionFeeResponse(properties);
                        };

                        /**
                         * Encodes the specified GetEstimatedTransactionFeeResponse message. Does not implicitly {@link org.dash.platform.dapi.v0.GetEstimatedTransactionFeeResponse.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.GetEstimatedTransactionFeeResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetEstimatedTransactionFeeResponse} message GetEstimatedTransactionFeeResponse message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        GetEstimatedTransactionFeeResponse.encode = function encode(message, writer) {
                            if (!writer)
                                writer = $Writer.create();
                            if (message.fee != null && Object.hasOwnProperty.call(message, "fee"))
                                writer.uint32(/* id 1, wireType 1 =*/9).double(message.fee);
                            return writer;
                        };

                        /**
                         * Encodes the specified GetEstimatedTransactionFeeResponse message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.GetEstimatedTransactionFeeResponse.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.GetEstimatedTransactionFeeResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetEstimatedTransactionFeeResponse} message GetEstimatedTransactionFeeResponse message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        GetEstimatedTransactionFeeResponse.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes a GetEstimatedTransactionFeeResponse message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.GetEstimatedTransactionFeeResponse
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.GetEstimatedTransactionFeeResponse} GetEstimatedTransactionFeeResponse
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        GetEstimatedTransactionFeeResponse.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.GetEstimatedTransactionFeeResponse();
                            while (reader.pos < end) {
                                var tag = reader.uint32();
                                switch (tag >>> 3) {
                                case 1:
                                    message.fee = reader.double();
                                    break;
                                default:
                                    reader.skipType(tag & 7);
                                    break;
                                }
                            }
                            return message;
                        };

                        /**
                         * Decodes a GetEstimatedTransactionFeeResponse message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.GetEstimatedTransactionFeeResponse
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.GetEstimatedTransactionFeeResponse} GetEstimatedTransactionFeeResponse
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        GetEstimatedTransactionFeeResponse.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies a GetEstimatedTransactionFeeResponse message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.GetEstimatedTransactionFeeResponse
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        GetEstimatedTransactionFeeResponse.verify = function verify(message) {
                            if (typeof message !== "object" || message === null)
                                return "object expected";
                            if (message.fee != null && message.hasOwnProperty("fee"))
                                if (typeof message.fee !== "number")
                                    return "fee: number expected";
                            return null;
                        };

                        /**
                         * Creates a GetEstimatedTransactionFeeResponse message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.GetEstimatedTransactionFeeResponse
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.GetEstimatedTransactionFeeResponse} GetEstimatedTransactionFeeResponse
                         */
                        GetEstimatedTransactionFeeResponse.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.GetEstimatedTransactionFeeResponse)
                                return object;
                            var message = new $root.org.dash.platform.dapi.v0.GetEstimatedTransactionFeeResponse();
                            if (object.fee != null)
                                message.fee = Number(object.fee);
                            return message;
                        };

                        /**
                         * Creates a plain object from a GetEstimatedTransactionFeeResponse message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.GetEstimatedTransactionFeeResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.GetEstimatedTransactionFeeResponse} message GetEstimatedTransactionFeeResponse
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        GetEstimatedTransactionFeeResponse.toObject = function toObject(message, options) {
                            if (!options)
                                options = {};
                            var object = {};
                            if (options.defaults)
                                object.fee = 0;
                            if (message.fee != null && message.hasOwnProperty("fee"))
                                object.fee = options.json && !isFinite(message.fee) ? String(message.fee) : message.fee;
                            return object;
                        };

                        /**
                         * Converts this GetEstimatedTransactionFeeResponse to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.GetEstimatedTransactionFeeResponse
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        GetEstimatedTransactionFeeResponse.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        return GetEstimatedTransactionFeeResponse;
                    })();

                    v0.TransactionsWithProofsRequest = (function() {

                        /**
                         * Properties of a TransactionsWithProofsRequest.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface ITransactionsWithProofsRequest
                         * @property {org.dash.platform.dapi.v0.IBloomFilter|null} [bloomFilter] TransactionsWithProofsRequest bloomFilter
                         * @property {Uint8Array|null} [fromBlockHash] TransactionsWithProofsRequest fromBlockHash
                         * @property {number|null} [fromBlockHeight] TransactionsWithProofsRequest fromBlockHeight
                         * @property {number|null} [count] TransactionsWithProofsRequest count
                         * @property {boolean|null} [sendTransactionHashes] TransactionsWithProofsRequest sendTransactionHashes
                         */

                        /**
                         * Constructs a new TransactionsWithProofsRequest.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a TransactionsWithProofsRequest.
                         * @implements ITransactionsWithProofsRequest
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.ITransactionsWithProofsRequest=} [properties] Properties to set
                         */
                        function TransactionsWithProofsRequest(properties) {
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * TransactionsWithProofsRequest bloomFilter.
                         * @member {org.dash.platform.dapi.v0.IBloomFilter|null|undefined} bloomFilter
                         * @memberof org.dash.platform.dapi.v0.TransactionsWithProofsRequest
                         * @instance
                         */
                        TransactionsWithProofsRequest.prototype.bloomFilter = null;

                        /**
                         * TransactionsWithProofsRequest fromBlockHash.
                         * @member {Uint8Array} fromBlockHash
                         * @memberof org.dash.platform.dapi.v0.TransactionsWithProofsRequest
                         * @instance
                         */
                        TransactionsWithProofsRequest.prototype.fromBlockHash = $util.newBuffer([]);

                        /**
                         * TransactionsWithProofsRequest fromBlockHeight.
                         * @member {number} fromBlockHeight
                         * @memberof org.dash.platform.dapi.v0.TransactionsWithProofsRequest
                         * @instance
                         */
                        TransactionsWithProofsRequest.prototype.fromBlockHeight = 0;

                        /**
                         * TransactionsWithProofsRequest count.
                         * @member {number} count
                         * @memberof org.dash.platform.dapi.v0.TransactionsWithProofsRequest
                         * @instance
                         */
                        TransactionsWithProofsRequest.prototype.count = 0;

                        /**
                         * TransactionsWithProofsRequest sendTransactionHashes.
                         * @member {boolean} sendTransactionHashes
                         * @memberof org.dash.platform.dapi.v0.TransactionsWithProofsRequest
                         * @instance
                         */
                        TransactionsWithProofsRequest.prototype.sendTransactionHashes = false;

                        // OneOf field names bound to virtual getters and setters
                        var $oneOfFields;

                        /**
                         * TransactionsWithProofsRequest fromBlock.
                         * @member {"fromBlockHash"|"fromBlockHeight"|undefined} fromBlock
                         * @memberof org.dash.platform.dapi.v0.TransactionsWithProofsRequest
                         * @instance
                         */
                        Object.defineProperty(TransactionsWithProofsRequest.prototype, "fromBlock", {
                            get: $util.oneOfGetter($oneOfFields = ["fromBlockHash", "fromBlockHeight"]),
                            set: $util.oneOfSetter($oneOfFields)
                        });

                        /**
                         * Creates a new TransactionsWithProofsRequest instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.TransactionsWithProofsRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.ITransactionsWithProofsRequest=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.TransactionsWithProofsRequest} TransactionsWithProofsRequest instance
                         */
                        TransactionsWithProofsRequest.create = function create(properties) {
                            return new TransactionsWithProofsRequest(properties);
                        };

                        /**
                         * Encodes the specified TransactionsWithProofsRequest message. Does not implicitly {@link org.dash.platform.dapi.v0.TransactionsWithProofsRequest.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.TransactionsWithProofsRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.ITransactionsWithProofsRequest} message TransactionsWithProofsRequest message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        TransactionsWithProofsRequest.encode = function encode(message, writer) {
                            if (!writer)
                                writer = $Writer.create();
                            if (message.bloomFilter != null && Object.hasOwnProperty.call(message, "bloomFilter"))
                                $root.org.dash.platform.dapi.v0.BloomFilter.encode(message.bloomFilter, writer.uint32(/* id 1, wireType 2 =*/10).fork()).ldelim();
                            if (message.fromBlockHash != null && Object.hasOwnProperty.call(message, "fromBlockHash"))
                                writer.uint32(/* id 2, wireType 2 =*/18).bytes(message.fromBlockHash);
                            if (message.fromBlockHeight != null && Object.hasOwnProperty.call(message, "fromBlockHeight"))
                                writer.uint32(/* id 3, wireType 0 =*/24).uint32(message.fromBlockHeight);
                            if (message.count != null && Object.hasOwnProperty.call(message, "count"))
                                writer.uint32(/* id 4, wireType 0 =*/32).uint32(message.count);
                            if (message.sendTransactionHashes != null && Object.hasOwnProperty.call(message, "sendTransactionHashes"))
                                writer.uint32(/* id 5, wireType 0 =*/40).bool(message.sendTransactionHashes);
                            return writer;
                        };

                        /**
                         * Encodes the specified TransactionsWithProofsRequest message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.TransactionsWithProofsRequest.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.TransactionsWithProofsRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.ITransactionsWithProofsRequest} message TransactionsWithProofsRequest message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        TransactionsWithProofsRequest.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes a TransactionsWithProofsRequest message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.TransactionsWithProofsRequest
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.TransactionsWithProofsRequest} TransactionsWithProofsRequest
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        TransactionsWithProofsRequest.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.TransactionsWithProofsRequest();
                            while (reader.pos < end) {
                                var tag = reader.uint32();
                                switch (tag >>> 3) {
                                case 1:
                                    message.bloomFilter = $root.org.dash.platform.dapi.v0.BloomFilter.decode(reader, reader.uint32());
                                    break;
                                case 2:
                                    message.fromBlockHash = reader.bytes();
                                    break;
                                case 3:
                                    message.fromBlockHeight = reader.uint32();
                                    break;
                                case 4:
                                    message.count = reader.uint32();
                                    break;
                                case 5:
                                    message.sendTransactionHashes = reader.bool();
                                    break;
                                default:
                                    reader.skipType(tag & 7);
                                    break;
                                }
                            }
                            return message;
                        };

                        /**
                         * Decodes a TransactionsWithProofsRequest message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.TransactionsWithProofsRequest
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.TransactionsWithProofsRequest} TransactionsWithProofsRequest
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        TransactionsWithProofsRequest.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies a TransactionsWithProofsRequest message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.TransactionsWithProofsRequest
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        TransactionsWithProofsRequest.verify = function verify(message) {
                            if (typeof message !== "object" || message === null)
                                return "object expected";
                            var properties = {};
                            if (message.bloomFilter != null && message.hasOwnProperty("bloomFilter")) {
                                var error = $root.org.dash.platform.dapi.v0.BloomFilter.verify(message.bloomFilter);
                                if (error)
                                    return "bloomFilter." + error;
                            }
                            if (message.fromBlockHash != null && message.hasOwnProperty("fromBlockHash")) {
                                properties.fromBlock = 1;
                                if (!(message.fromBlockHash && typeof message.fromBlockHash.length === "number" || $util.isString(message.fromBlockHash)))
                                    return "fromBlockHash: buffer expected";
                            }
                            if (message.fromBlockHeight != null && message.hasOwnProperty("fromBlockHeight")) {
                                if (properties.fromBlock === 1)
                                    return "fromBlock: multiple values";
                                properties.fromBlock = 1;
                                if (!$util.isInteger(message.fromBlockHeight))
                                    return "fromBlockHeight: integer expected";
                            }
                            if (message.count != null && message.hasOwnProperty("count"))
                                if (!$util.isInteger(message.count))
                                    return "count: integer expected";
                            if (message.sendTransactionHashes != null && message.hasOwnProperty("sendTransactionHashes"))
                                if (typeof message.sendTransactionHashes !== "boolean")
                                    return "sendTransactionHashes: boolean expected";
                            return null;
                        };

                        /**
                         * Creates a TransactionsWithProofsRequest message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.TransactionsWithProofsRequest
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.TransactionsWithProofsRequest} TransactionsWithProofsRequest
                         */
                        TransactionsWithProofsRequest.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.TransactionsWithProofsRequest)
                                return object;
                            var message = new $root.org.dash.platform.dapi.v0.TransactionsWithProofsRequest();
                            if (object.bloomFilter != null) {
                                if (typeof object.bloomFilter !== "object")
                                    throw TypeError(".org.dash.platform.dapi.v0.TransactionsWithProofsRequest.bloomFilter: object expected");
                                message.bloomFilter = $root.org.dash.platform.dapi.v0.BloomFilter.fromObject(object.bloomFilter);
                            }
                            if (object.fromBlockHash != null)
                                if (typeof object.fromBlockHash === "string")
                                    $util.base64.decode(object.fromBlockHash, message.fromBlockHash = $util.newBuffer($util.base64.length(object.fromBlockHash)), 0);
                                else if (object.fromBlockHash.length >= 0)
                                    message.fromBlockHash = object.fromBlockHash;
                            if (object.fromBlockHeight != null)
                                message.fromBlockHeight = object.fromBlockHeight >>> 0;
                            if (object.count != null)
                                message.count = object.count >>> 0;
                            if (object.sendTransactionHashes != null)
                                message.sendTransactionHashes = Boolean(object.sendTransactionHashes);
                            return message;
                        };

                        /**
                         * Creates a plain object from a TransactionsWithProofsRequest message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.TransactionsWithProofsRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.TransactionsWithProofsRequest} message TransactionsWithProofsRequest
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        TransactionsWithProofsRequest.toObject = function toObject(message, options) {
                            if (!options)
                                options = {};
                            var object = {};
                            if (options.defaults) {
                                object.bloomFilter = null;
                                object.count = 0;
                                object.sendTransactionHashes = false;
                            }
                            if (message.bloomFilter != null && message.hasOwnProperty("bloomFilter"))
                                object.bloomFilter = $root.org.dash.platform.dapi.v0.BloomFilter.toObject(message.bloomFilter, options);
                            if (message.fromBlockHash != null && message.hasOwnProperty("fromBlockHash")) {
                                object.fromBlockHash = options.bytes === String ? $util.base64.encode(message.fromBlockHash, 0, message.fromBlockHash.length) : options.bytes === Array ? Array.prototype.slice.call(message.fromBlockHash) : message.fromBlockHash;
                                if (options.oneofs)
                                    object.fromBlock = "fromBlockHash";
                            }
                            if (message.fromBlockHeight != null && message.hasOwnProperty("fromBlockHeight")) {
                                object.fromBlockHeight = message.fromBlockHeight;
                                if (options.oneofs)
                                    object.fromBlock = "fromBlockHeight";
                            }
                            if (message.count != null && message.hasOwnProperty("count"))
                                object.count = message.count;
                            if (message.sendTransactionHashes != null && message.hasOwnProperty("sendTransactionHashes"))
                                object.sendTransactionHashes = message.sendTransactionHashes;
                            return object;
                        };

                        /**
                         * Converts this TransactionsWithProofsRequest to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.TransactionsWithProofsRequest
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        TransactionsWithProofsRequest.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        return TransactionsWithProofsRequest;
                    })();

                    v0.BloomFilter = (function() {

                        /**
                         * Properties of a BloomFilter.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface IBloomFilter
                         * @property {Uint8Array|null} [vData] BloomFilter vData
                         * @property {number|null} [nHashFuncs] BloomFilter nHashFuncs
                         * @property {number|null} [nTweak] BloomFilter nTweak
                         * @property {number|null} [nFlags] BloomFilter nFlags
                         */

                        /**
                         * Constructs a new BloomFilter.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a BloomFilter.
                         * @implements IBloomFilter
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.IBloomFilter=} [properties] Properties to set
                         */
                        function BloomFilter(properties) {
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * BloomFilter vData.
                         * @member {Uint8Array} vData
                         * @memberof org.dash.platform.dapi.v0.BloomFilter
                         * @instance
                         */
                        BloomFilter.prototype.vData = $util.newBuffer([]);

                        /**
                         * BloomFilter nHashFuncs.
                         * @member {number} nHashFuncs
                         * @memberof org.dash.platform.dapi.v0.BloomFilter
                         * @instance
                         */
                        BloomFilter.prototype.nHashFuncs = 0;

                        /**
                         * BloomFilter nTweak.
                         * @member {number} nTweak
                         * @memberof org.dash.platform.dapi.v0.BloomFilter
                         * @instance
                         */
                        BloomFilter.prototype.nTweak = 0;

                        /**
                         * BloomFilter nFlags.
                         * @member {number} nFlags
                         * @memberof org.dash.platform.dapi.v0.BloomFilter
                         * @instance
                         */
                        BloomFilter.prototype.nFlags = 0;

                        /**
                         * Creates a new BloomFilter instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.BloomFilter
                         * @static
                         * @param {org.dash.platform.dapi.v0.IBloomFilter=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.BloomFilter} BloomFilter instance
                         */
                        BloomFilter.create = function create(properties) {
                            return new BloomFilter(properties);
                        };

                        /**
                         * Encodes the specified BloomFilter message. Does not implicitly {@link org.dash.platform.dapi.v0.BloomFilter.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.BloomFilter
                         * @static
                         * @param {org.dash.platform.dapi.v0.IBloomFilter} message BloomFilter message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        BloomFilter.encode = function encode(message, writer) {
                            if (!writer)
                                writer = $Writer.create();
                            if (message.vData != null && Object.hasOwnProperty.call(message, "vData"))
                                writer.uint32(/* id 1, wireType 2 =*/10).bytes(message.vData);
                            if (message.nHashFuncs != null && Object.hasOwnProperty.call(message, "nHashFuncs"))
                                writer.uint32(/* id 2, wireType 0 =*/16).uint32(message.nHashFuncs);
                            if (message.nTweak != null && Object.hasOwnProperty.call(message, "nTweak"))
                                writer.uint32(/* id 3, wireType 0 =*/24).uint32(message.nTweak);
                            if (message.nFlags != null && Object.hasOwnProperty.call(message, "nFlags"))
                                writer.uint32(/* id 4, wireType 0 =*/32).uint32(message.nFlags);
                            return writer;
                        };

                        /**
                         * Encodes the specified BloomFilter message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.BloomFilter.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.BloomFilter
                         * @static
                         * @param {org.dash.platform.dapi.v0.IBloomFilter} message BloomFilter message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        BloomFilter.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes a BloomFilter message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.BloomFilter
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.BloomFilter} BloomFilter
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        BloomFilter.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.BloomFilter();
                            while (reader.pos < end) {
                                var tag = reader.uint32();
                                switch (tag >>> 3) {
                                case 1:
                                    message.vData = reader.bytes();
                                    break;
                                case 2:
                                    message.nHashFuncs = reader.uint32();
                                    break;
                                case 3:
                                    message.nTweak = reader.uint32();
                                    break;
                                case 4:
                                    message.nFlags = reader.uint32();
                                    break;
                                default:
                                    reader.skipType(tag & 7);
                                    break;
                                }
                            }
                            return message;
                        };

                        /**
                         * Decodes a BloomFilter message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.BloomFilter
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.BloomFilter} BloomFilter
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        BloomFilter.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies a BloomFilter message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.BloomFilter
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        BloomFilter.verify = function verify(message) {
                            if (typeof message !== "object" || message === null)
                                return "object expected";
                            if (message.vData != null && message.hasOwnProperty("vData"))
                                if (!(message.vData && typeof message.vData.length === "number" || $util.isString(message.vData)))
                                    return "vData: buffer expected";
                            if (message.nHashFuncs != null && message.hasOwnProperty("nHashFuncs"))
                                if (!$util.isInteger(message.nHashFuncs))
                                    return "nHashFuncs: integer expected";
                            if (message.nTweak != null && message.hasOwnProperty("nTweak"))
                                if (!$util.isInteger(message.nTweak))
                                    return "nTweak: integer expected";
                            if (message.nFlags != null && message.hasOwnProperty("nFlags"))
                                if (!$util.isInteger(message.nFlags))
                                    return "nFlags: integer expected";
                            return null;
                        };

                        /**
                         * Creates a BloomFilter message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.BloomFilter
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.BloomFilter} BloomFilter
                         */
                        BloomFilter.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.BloomFilter)
                                return object;
                            var message = new $root.org.dash.platform.dapi.v0.BloomFilter();
                            if (object.vData != null)
                                if (typeof object.vData === "string")
                                    $util.base64.decode(object.vData, message.vData = $util.newBuffer($util.base64.length(object.vData)), 0);
                                else if (object.vData.length >= 0)
                                    message.vData = object.vData;
                            if (object.nHashFuncs != null)
                                message.nHashFuncs = object.nHashFuncs >>> 0;
                            if (object.nTweak != null)
                                message.nTweak = object.nTweak >>> 0;
                            if (object.nFlags != null)
                                message.nFlags = object.nFlags >>> 0;
                            return message;
                        };

                        /**
                         * Creates a plain object from a BloomFilter message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.BloomFilter
                         * @static
                         * @param {org.dash.platform.dapi.v0.BloomFilter} message BloomFilter
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        BloomFilter.toObject = function toObject(message, options) {
                            if (!options)
                                options = {};
                            var object = {};
                            if (options.defaults) {
                                if (options.bytes === String)
                                    object.vData = "";
                                else {
                                    object.vData = [];
                                    if (options.bytes !== Array)
                                        object.vData = $util.newBuffer(object.vData);
                                }
                                object.nHashFuncs = 0;
                                object.nTweak = 0;
                                object.nFlags = 0;
                            }
                            if (message.vData != null && message.hasOwnProperty("vData"))
                                object.vData = options.bytes === String ? $util.base64.encode(message.vData, 0, message.vData.length) : options.bytes === Array ? Array.prototype.slice.call(message.vData) : message.vData;
                            if (message.nHashFuncs != null && message.hasOwnProperty("nHashFuncs"))
                                object.nHashFuncs = message.nHashFuncs;
                            if (message.nTweak != null && message.hasOwnProperty("nTweak"))
                                object.nTweak = message.nTweak;
                            if (message.nFlags != null && message.hasOwnProperty("nFlags"))
                                object.nFlags = message.nFlags;
                            return object;
                        };

                        /**
                         * Converts this BloomFilter to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.BloomFilter
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        BloomFilter.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        return BloomFilter;
                    })();

                    v0.TransactionsWithProofsResponse = (function() {

                        /**
                         * Properties of a TransactionsWithProofsResponse.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface ITransactionsWithProofsResponse
                         * @property {org.dash.platform.dapi.v0.IRawTransactions|null} [rawTransactions] TransactionsWithProofsResponse rawTransactions
                         * @property {org.dash.platform.dapi.v0.IInstantSendLockMessages|null} [instantSendLockMessages] TransactionsWithProofsResponse instantSendLockMessages
                         * @property {Uint8Array|null} [rawMerkleBlock] TransactionsWithProofsResponse rawMerkleBlock
                         */

                        /**
                         * Constructs a new TransactionsWithProofsResponse.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a TransactionsWithProofsResponse.
                         * @implements ITransactionsWithProofsResponse
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.ITransactionsWithProofsResponse=} [properties] Properties to set
                         */
                        function TransactionsWithProofsResponse(properties) {
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * TransactionsWithProofsResponse rawTransactions.
                         * @member {org.dash.platform.dapi.v0.IRawTransactions|null|undefined} rawTransactions
                         * @memberof org.dash.platform.dapi.v0.TransactionsWithProofsResponse
                         * @instance
                         */
                        TransactionsWithProofsResponse.prototype.rawTransactions = null;

                        /**
                         * TransactionsWithProofsResponse instantSendLockMessages.
                         * @member {org.dash.platform.dapi.v0.IInstantSendLockMessages|null|undefined} instantSendLockMessages
                         * @memberof org.dash.platform.dapi.v0.TransactionsWithProofsResponse
                         * @instance
                         */
                        TransactionsWithProofsResponse.prototype.instantSendLockMessages = null;

                        /**
                         * TransactionsWithProofsResponse rawMerkleBlock.
                         * @member {Uint8Array} rawMerkleBlock
                         * @memberof org.dash.platform.dapi.v0.TransactionsWithProofsResponse
                         * @instance
                         */
                        TransactionsWithProofsResponse.prototype.rawMerkleBlock = $util.newBuffer([]);

                        // OneOf field names bound to virtual getters and setters
                        var $oneOfFields;

                        /**
                         * TransactionsWithProofsResponse responses.
                         * @member {"rawTransactions"|"instantSendLockMessages"|"rawMerkleBlock"|undefined} responses
                         * @memberof org.dash.platform.dapi.v0.TransactionsWithProofsResponse
                         * @instance
                         */
                        Object.defineProperty(TransactionsWithProofsResponse.prototype, "responses", {
                            get: $util.oneOfGetter($oneOfFields = ["rawTransactions", "instantSendLockMessages", "rawMerkleBlock"]),
                            set: $util.oneOfSetter($oneOfFields)
                        });

                        /**
                         * Creates a new TransactionsWithProofsResponse instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.TransactionsWithProofsResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.ITransactionsWithProofsResponse=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.TransactionsWithProofsResponse} TransactionsWithProofsResponse instance
                         */
                        TransactionsWithProofsResponse.create = function create(properties) {
                            return new TransactionsWithProofsResponse(properties);
                        };

                        /**
                         * Encodes the specified TransactionsWithProofsResponse message. Does not implicitly {@link org.dash.platform.dapi.v0.TransactionsWithProofsResponse.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.TransactionsWithProofsResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.ITransactionsWithProofsResponse} message TransactionsWithProofsResponse message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        TransactionsWithProofsResponse.encode = function encode(message, writer) {
                            if (!writer)
                                writer = $Writer.create();
                            if (message.rawTransactions != null && Object.hasOwnProperty.call(message, "rawTransactions"))
                                $root.org.dash.platform.dapi.v0.RawTransactions.encode(message.rawTransactions, writer.uint32(/* id 1, wireType 2 =*/10).fork()).ldelim();
                            if (message.instantSendLockMessages != null && Object.hasOwnProperty.call(message, "instantSendLockMessages"))
                                $root.org.dash.platform.dapi.v0.InstantSendLockMessages.encode(message.instantSendLockMessages, writer.uint32(/* id 2, wireType 2 =*/18).fork()).ldelim();
                            if (message.rawMerkleBlock != null && Object.hasOwnProperty.call(message, "rawMerkleBlock"))
                                writer.uint32(/* id 3, wireType 2 =*/26).bytes(message.rawMerkleBlock);
                            return writer;
                        };

                        /**
                         * Encodes the specified TransactionsWithProofsResponse message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.TransactionsWithProofsResponse.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.TransactionsWithProofsResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.ITransactionsWithProofsResponse} message TransactionsWithProofsResponse message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        TransactionsWithProofsResponse.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes a TransactionsWithProofsResponse message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.TransactionsWithProofsResponse
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.TransactionsWithProofsResponse} TransactionsWithProofsResponse
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        TransactionsWithProofsResponse.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.TransactionsWithProofsResponse();
                            while (reader.pos < end) {
                                var tag = reader.uint32();
                                switch (tag >>> 3) {
                                case 1:
                                    message.rawTransactions = $root.org.dash.platform.dapi.v0.RawTransactions.decode(reader, reader.uint32());
                                    break;
                                case 2:
                                    message.instantSendLockMessages = $root.org.dash.platform.dapi.v0.InstantSendLockMessages.decode(reader, reader.uint32());
                                    break;
                                case 3:
                                    message.rawMerkleBlock = reader.bytes();
                                    break;
                                default:
                                    reader.skipType(tag & 7);
                                    break;
                                }
                            }
                            return message;
                        };

                        /**
                         * Decodes a TransactionsWithProofsResponse message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.TransactionsWithProofsResponse
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.TransactionsWithProofsResponse} TransactionsWithProofsResponse
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        TransactionsWithProofsResponse.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies a TransactionsWithProofsResponse message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.TransactionsWithProofsResponse
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        TransactionsWithProofsResponse.verify = function verify(message) {
                            if (typeof message !== "object" || message === null)
                                return "object expected";
                            var properties = {};
                            if (message.rawTransactions != null && message.hasOwnProperty("rawTransactions")) {
                                properties.responses = 1;
                                {
                                    var error = $root.org.dash.platform.dapi.v0.RawTransactions.verify(message.rawTransactions);
                                    if (error)
                                        return "rawTransactions." + error;
                                }
                            }
                            if (message.instantSendLockMessages != null && message.hasOwnProperty("instantSendLockMessages")) {
                                if (properties.responses === 1)
                                    return "responses: multiple values";
                                properties.responses = 1;
                                {
                                    var error = $root.org.dash.platform.dapi.v0.InstantSendLockMessages.verify(message.instantSendLockMessages);
                                    if (error)
                                        return "instantSendLockMessages." + error;
                                }
                            }
                            if (message.rawMerkleBlock != null && message.hasOwnProperty("rawMerkleBlock")) {
                                if (properties.responses === 1)
                                    return "responses: multiple values";
                                properties.responses = 1;
                                if (!(message.rawMerkleBlock && typeof message.rawMerkleBlock.length === "number" || $util.isString(message.rawMerkleBlock)))
                                    return "rawMerkleBlock: buffer expected";
                            }
                            return null;
                        };

                        /**
                         * Creates a TransactionsWithProofsResponse message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.TransactionsWithProofsResponse
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.TransactionsWithProofsResponse} TransactionsWithProofsResponse
                         */
                        TransactionsWithProofsResponse.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.TransactionsWithProofsResponse)
                                return object;
                            var message = new $root.org.dash.platform.dapi.v0.TransactionsWithProofsResponse();
                            if (object.rawTransactions != null) {
                                if (typeof object.rawTransactions !== "object")
                                    throw TypeError(".org.dash.platform.dapi.v0.TransactionsWithProofsResponse.rawTransactions: object expected");
                                message.rawTransactions = $root.org.dash.platform.dapi.v0.RawTransactions.fromObject(object.rawTransactions);
                            }
                            if (object.instantSendLockMessages != null) {
                                if (typeof object.instantSendLockMessages !== "object")
                                    throw TypeError(".org.dash.platform.dapi.v0.TransactionsWithProofsResponse.instantSendLockMessages: object expected");
                                message.instantSendLockMessages = $root.org.dash.platform.dapi.v0.InstantSendLockMessages.fromObject(object.instantSendLockMessages);
                            }
                            if (object.rawMerkleBlock != null)
                                if (typeof object.rawMerkleBlock === "string")
                                    $util.base64.decode(object.rawMerkleBlock, message.rawMerkleBlock = $util.newBuffer($util.base64.length(object.rawMerkleBlock)), 0);
                                else if (object.rawMerkleBlock.length >= 0)
                                    message.rawMerkleBlock = object.rawMerkleBlock;
                            return message;
                        };

                        /**
                         * Creates a plain object from a TransactionsWithProofsResponse message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.TransactionsWithProofsResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.TransactionsWithProofsResponse} message TransactionsWithProofsResponse
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        TransactionsWithProofsResponse.toObject = function toObject(message, options) {
                            if (!options)
                                options = {};
                            var object = {};
                            if (message.rawTransactions != null && message.hasOwnProperty("rawTransactions")) {
                                object.rawTransactions = $root.org.dash.platform.dapi.v0.RawTransactions.toObject(message.rawTransactions, options);
                                if (options.oneofs)
                                    object.responses = "rawTransactions";
                            }
                            if (message.instantSendLockMessages != null && message.hasOwnProperty("instantSendLockMessages")) {
                                object.instantSendLockMessages = $root.org.dash.platform.dapi.v0.InstantSendLockMessages.toObject(message.instantSendLockMessages, options);
                                if (options.oneofs)
                                    object.responses = "instantSendLockMessages";
                            }
                            if (message.rawMerkleBlock != null && message.hasOwnProperty("rawMerkleBlock")) {
                                object.rawMerkleBlock = options.bytes === String ? $util.base64.encode(message.rawMerkleBlock, 0, message.rawMerkleBlock.length) : options.bytes === Array ? Array.prototype.slice.call(message.rawMerkleBlock) : message.rawMerkleBlock;
                                if (options.oneofs)
                                    object.responses = "rawMerkleBlock";
                            }
                            return object;
                        };

                        /**
                         * Converts this TransactionsWithProofsResponse to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.TransactionsWithProofsResponse
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        TransactionsWithProofsResponse.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        return TransactionsWithProofsResponse;
                    })();

                    v0.RawTransactions = (function() {

                        /**
                         * Properties of a RawTransactions.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface IRawTransactions
                         * @property {Array.<Uint8Array>|null} [transactions] RawTransactions transactions
                         */

                        /**
                         * Constructs a new RawTransactions.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a RawTransactions.
                         * @implements IRawTransactions
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.IRawTransactions=} [properties] Properties to set
                         */
                        function RawTransactions(properties) {
                            this.transactions = [];
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * RawTransactions transactions.
                         * @member {Array.<Uint8Array>} transactions
                         * @memberof org.dash.platform.dapi.v0.RawTransactions
                         * @instance
                         */
                        RawTransactions.prototype.transactions = $util.emptyArray;

                        /**
                         * Creates a new RawTransactions instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.RawTransactions
                         * @static
                         * @param {org.dash.platform.dapi.v0.IRawTransactions=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.RawTransactions} RawTransactions instance
                         */
                        RawTransactions.create = function create(properties) {
                            return new RawTransactions(properties);
                        };

                        /**
                         * Encodes the specified RawTransactions message. Does not implicitly {@link org.dash.platform.dapi.v0.RawTransactions.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.RawTransactions
                         * @static
                         * @param {org.dash.platform.dapi.v0.IRawTransactions} message RawTransactions message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        RawTransactions.encode = function encode(message, writer) {
                            if (!writer)
                                writer = $Writer.create();
                            if (message.transactions != null && message.transactions.length)
                                for (var i = 0; i < message.transactions.length; ++i)
                                    writer.uint32(/* id 1, wireType 2 =*/10).bytes(message.transactions[i]);
                            return writer;
                        };

                        /**
                         * Encodes the specified RawTransactions message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.RawTransactions.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.RawTransactions
                         * @static
                         * @param {org.dash.platform.dapi.v0.IRawTransactions} message RawTransactions message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        RawTransactions.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes a RawTransactions message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.RawTransactions
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.RawTransactions} RawTransactions
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        RawTransactions.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.RawTransactions();
                            while (reader.pos < end) {
                                var tag = reader.uint32();
                                switch (tag >>> 3) {
                                case 1:
                                    if (!(message.transactions && message.transactions.length))
                                        message.transactions = [];
                                    message.transactions.push(reader.bytes());
                                    break;
                                default:
                                    reader.skipType(tag & 7);
                                    break;
                                }
                            }
                            return message;
                        };

                        /**
                         * Decodes a RawTransactions message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.RawTransactions
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.RawTransactions} RawTransactions
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        RawTransactions.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies a RawTransactions message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.RawTransactions
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        RawTransactions.verify = function verify(message) {
                            if (typeof message !== "object" || message === null)
                                return "object expected";
                            if (message.transactions != null && message.hasOwnProperty("transactions")) {
                                if (!Array.isArray(message.transactions))
                                    return "transactions: array expected";
                                for (var i = 0; i < message.transactions.length; ++i)
                                    if (!(message.transactions[i] && typeof message.transactions[i].length === "number" || $util.isString(message.transactions[i])))
                                        return "transactions: buffer[] expected";
                            }
                            return null;
                        };

                        /**
                         * Creates a RawTransactions message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.RawTransactions
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.RawTransactions} RawTransactions
                         */
                        RawTransactions.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.RawTransactions)
                                return object;
                            var message = new $root.org.dash.platform.dapi.v0.RawTransactions();
                            if (object.transactions) {
                                if (!Array.isArray(object.transactions))
                                    throw TypeError(".org.dash.platform.dapi.v0.RawTransactions.transactions: array expected");
                                message.transactions = [];
                                for (var i = 0; i < object.transactions.length; ++i)
                                    if (typeof object.transactions[i] === "string")
                                        $util.base64.decode(object.transactions[i], message.transactions[i] = $util.newBuffer($util.base64.length(object.transactions[i])), 0);
                                    else if (object.transactions[i].length >= 0)
                                        message.transactions[i] = object.transactions[i];
                            }
                            return message;
                        };

                        /**
                         * Creates a plain object from a RawTransactions message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.RawTransactions
                         * @static
                         * @param {org.dash.platform.dapi.v0.RawTransactions} message RawTransactions
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        RawTransactions.toObject = function toObject(message, options) {
                            if (!options)
                                options = {};
                            var object = {};
                            if (options.arrays || options.defaults)
                                object.transactions = [];
                            if (message.transactions && message.transactions.length) {
                                object.transactions = [];
                                for (var j = 0; j < message.transactions.length; ++j)
                                    object.transactions[j] = options.bytes === String ? $util.base64.encode(message.transactions[j], 0, message.transactions[j].length) : options.bytes === Array ? Array.prototype.slice.call(message.transactions[j]) : message.transactions[j];
                            }
                            return object;
                        };

                        /**
                         * Converts this RawTransactions to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.RawTransactions
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        RawTransactions.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        return RawTransactions;
                    })();

                    v0.InstantSendLockMessages = (function() {

                        /**
                         * Properties of an InstantSendLockMessages.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface IInstantSendLockMessages
                         * @property {Array.<Uint8Array>|null} [messages] InstantSendLockMessages messages
                         */

                        /**
                         * Constructs a new InstantSendLockMessages.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents an InstantSendLockMessages.
                         * @implements IInstantSendLockMessages
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.IInstantSendLockMessages=} [properties] Properties to set
                         */
                        function InstantSendLockMessages(properties) {
                            this.messages = [];
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * InstantSendLockMessages messages.
                         * @member {Array.<Uint8Array>} messages
                         * @memberof org.dash.platform.dapi.v0.InstantSendLockMessages
                         * @instance
                         */
                        InstantSendLockMessages.prototype.messages = $util.emptyArray;

                        /**
                         * Creates a new InstantSendLockMessages instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.InstantSendLockMessages
                         * @static
                         * @param {org.dash.platform.dapi.v0.IInstantSendLockMessages=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.InstantSendLockMessages} InstantSendLockMessages instance
                         */
                        InstantSendLockMessages.create = function create(properties) {
                            return new InstantSendLockMessages(properties);
                        };

                        /**
                         * Encodes the specified InstantSendLockMessages message. Does not implicitly {@link org.dash.platform.dapi.v0.InstantSendLockMessages.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.InstantSendLockMessages
                         * @static
                         * @param {org.dash.platform.dapi.v0.IInstantSendLockMessages} message InstantSendLockMessages message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        InstantSendLockMessages.encode = function encode(message, writer) {
                            if (!writer)
                                writer = $Writer.create();
                            if (message.messages != null && message.messages.length)
                                for (var i = 0; i < message.messages.length; ++i)
                                    writer.uint32(/* id 1, wireType 2 =*/10).bytes(message.messages[i]);
                            return writer;
                        };

                        /**
                         * Encodes the specified InstantSendLockMessages message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.InstantSendLockMessages.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.InstantSendLockMessages
                         * @static
                         * @param {org.dash.platform.dapi.v0.IInstantSendLockMessages} message InstantSendLockMessages message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        InstantSendLockMessages.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes an InstantSendLockMessages message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.InstantSendLockMessages
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.InstantSendLockMessages} InstantSendLockMessages
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        InstantSendLockMessages.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.InstantSendLockMessages();
                            while (reader.pos < end) {
                                var tag = reader.uint32();
                                switch (tag >>> 3) {
                                case 1:
                                    if (!(message.messages && message.messages.length))
                                        message.messages = [];
                                    message.messages.push(reader.bytes());
                                    break;
                                default:
                                    reader.skipType(tag & 7);
                                    break;
                                }
                            }
                            return message;
                        };

                        /**
                         * Decodes an InstantSendLockMessages message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.InstantSendLockMessages
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.InstantSendLockMessages} InstantSendLockMessages
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        InstantSendLockMessages.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies an InstantSendLockMessages message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.InstantSendLockMessages
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        InstantSendLockMessages.verify = function verify(message) {
                            if (typeof message !== "object" || message === null)
                                return "object expected";
                            if (message.messages != null && message.hasOwnProperty("messages")) {
                                if (!Array.isArray(message.messages))
                                    return "messages: array expected";
                                for (var i = 0; i < message.messages.length; ++i)
                                    if (!(message.messages[i] && typeof message.messages[i].length === "number" || $util.isString(message.messages[i])))
                                        return "messages: buffer[] expected";
                            }
                            return null;
                        };

                        /**
                         * Creates an InstantSendLockMessages message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.InstantSendLockMessages
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.InstantSendLockMessages} InstantSendLockMessages
                         */
                        InstantSendLockMessages.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.InstantSendLockMessages)
                                return object;
                            var message = new $root.org.dash.platform.dapi.v0.InstantSendLockMessages();
                            if (object.messages) {
                                if (!Array.isArray(object.messages))
                                    throw TypeError(".org.dash.platform.dapi.v0.InstantSendLockMessages.messages: array expected");
                                message.messages = [];
                                for (var i = 0; i < object.messages.length; ++i)
                                    if (typeof object.messages[i] === "string")
                                        $util.base64.decode(object.messages[i], message.messages[i] = $util.newBuffer($util.base64.length(object.messages[i])), 0);
                                    else if (object.messages[i].length >= 0)
                                        message.messages[i] = object.messages[i];
                            }
                            return message;
                        };

                        /**
                         * Creates a plain object from an InstantSendLockMessages message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.InstantSendLockMessages
                         * @static
                         * @param {org.dash.platform.dapi.v0.InstantSendLockMessages} message InstantSendLockMessages
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        InstantSendLockMessages.toObject = function toObject(message, options) {
                            if (!options)
                                options = {};
                            var object = {};
                            if (options.arrays || options.defaults)
                                object.messages = [];
                            if (message.messages && message.messages.length) {
                                object.messages = [];
                                for (var j = 0; j < message.messages.length; ++j)
                                    object.messages[j] = options.bytes === String ? $util.base64.encode(message.messages[j], 0, message.messages[j].length) : options.bytes === Array ? Array.prototype.slice.call(message.messages[j]) : message.messages[j];
                            }
                            return object;
                        };

                        /**
                         * Converts this InstantSendLockMessages to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.InstantSendLockMessages
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        InstantSendLockMessages.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        return InstantSendLockMessages;
                    })();

                    return v0;
                })();

                return dapi;
            })();

            return platform;
        })();

        return dash;
    })();

    return org;
})();

module.exports = $root;
