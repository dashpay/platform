/*eslint-disable block-scoped-var, id-length, no-control-regex, no-magic-numbers, no-prototype-builtins, no-redeclare, no-shadow, no-var, sort-vars*/
"use strict";

var $protobuf = require("@dashevo/protobufjs/minimal");

// Common aliases
var $Reader = $protobuf.Reader, $Writer = $protobuf.Writer, $util = $protobuf.util;

// Exported root namespace
var $root = $protobuf.roots.platform_root || ($protobuf.roots.platform_root = {});

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

                    v0.Platform = (function() {

                        /**
                         * Constructs a new Platform service.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a Platform
                         * @extends $protobuf.rpc.Service
                         * @constructor
                         * @param {$protobuf.RPCImpl} rpcImpl RPC implementation
                         * @param {boolean} [requestDelimited=false] Whether requests are length-delimited
                         * @param {boolean} [responseDelimited=false] Whether responses are length-delimited
                         */
                        function Platform(rpcImpl, requestDelimited, responseDelimited) {
                            $protobuf.rpc.Service.call(this, rpcImpl, requestDelimited, responseDelimited);
                        }

                        (Platform.prototype = Object.create($protobuf.rpc.Service.prototype)).constructor = Platform;

                        /**
                         * Creates new Platform service using the specified rpc implementation.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.Platform
                         * @static
                         * @param {$protobuf.RPCImpl} rpcImpl RPC implementation
                         * @param {boolean} [requestDelimited=false] Whether requests are length-delimited
                         * @param {boolean} [responseDelimited=false] Whether responses are length-delimited
                         * @returns {Platform} RPC service. Useful where requests and/or responses are streamed.
                         */
                        Platform.create = function create(rpcImpl, requestDelimited, responseDelimited) {
                            return new this(rpcImpl, requestDelimited, responseDelimited);
                        };

                        /**
                         * Callback as used by {@link org.dash.platform.dapi.v0.Platform#broadcastStateTransition}.
                         * @memberof org.dash.platform.dapi.v0.Platform
                         * @typedef broadcastStateTransitionCallback
                         * @type {function}
                         * @param {Error|null} error Error, if any
                         * @param {org.dash.platform.dapi.v0.BroadcastStateTransitionResponse} [response] BroadcastStateTransitionResponse
                         */

                        /**
                         * Calls broadcastStateTransition.
                         * @function broadcastStateTransition
                         * @memberof org.dash.platform.dapi.v0.Platform
                         * @instance
                         * @param {org.dash.platform.dapi.v0.IBroadcastStateTransitionRequest} request BroadcastStateTransitionRequest message or plain object
                         * @param {org.dash.platform.dapi.v0.Platform.broadcastStateTransitionCallback} callback Node-style callback called with the error, if any, and BroadcastStateTransitionResponse
                         * @returns {undefined}
                         * @variation 1
                         */
                        Object.defineProperty(Platform.prototype.broadcastStateTransition = function broadcastStateTransition(request, callback) {
                            return this.rpcCall(broadcastStateTransition, $root.org.dash.platform.dapi.v0.BroadcastStateTransitionRequest, $root.org.dash.platform.dapi.v0.BroadcastStateTransitionResponse, request, callback);
                        }, "name", { value: "broadcastStateTransition" });

                        /**
                         * Calls broadcastStateTransition.
                         * @function broadcastStateTransition
                         * @memberof org.dash.platform.dapi.v0.Platform
                         * @instance
                         * @param {org.dash.platform.dapi.v0.IBroadcastStateTransitionRequest} request BroadcastStateTransitionRequest message or plain object
                         * @returns {Promise<org.dash.platform.dapi.v0.BroadcastStateTransitionResponse>} Promise
                         * @variation 2
                         */

                        /**
                         * Callback as used by {@link org.dash.platform.dapi.v0.Platform#getIdentity}.
                         * @memberof org.dash.platform.dapi.v0.Platform
                         * @typedef getIdentityCallback
                         * @type {function}
                         * @param {Error|null} error Error, if any
                         * @param {org.dash.platform.dapi.v0.SingleItemResponse} [response] SingleItemResponse
                         */

                        /**
                         * Calls getIdentity.
                         * @function getIdentity
                         * @memberof org.dash.platform.dapi.v0.Platform
                         * @instance
                         * @param {org.dash.platform.dapi.v0.IGetSingleItemRequest} request GetSingleItemRequest message or plain object
                         * @param {org.dash.platform.dapi.v0.Platform.getIdentityCallback} callback Node-style callback called with the error, if any, and SingleItemResponse
                         * @returns {undefined}
                         * @variation 1
                         */
                        Object.defineProperty(Platform.prototype.getIdentity = function getIdentity(request, callback) {
                            return this.rpcCall(getIdentity, $root.org.dash.platform.dapi.v0.GetSingleItemRequest, $root.org.dash.platform.dapi.v0.SingleItemResponse, request, callback);
                        }, "name", { value: "getIdentity" });

                        /**
                         * Calls getIdentity.
                         * @function getIdentity
                         * @memberof org.dash.platform.dapi.v0.Platform
                         * @instance
                         * @param {org.dash.platform.dapi.v0.IGetSingleItemRequest} request GetSingleItemRequest message or plain object
                         * @returns {Promise<org.dash.platform.dapi.v0.SingleItemResponse>} Promise
                         * @variation 2
                         */

                        /**
                         * Callback as used by {@link org.dash.platform.dapi.v0.Platform#getIdentityBalance}.
                         * @memberof org.dash.platform.dapi.v0.Platform
                         * @typedef getIdentityBalanceCallback
                         * @type {function}
                         * @param {Error|null} error Error, if any
                         * @param {org.dash.platform.dapi.v0.SingleItemResponse} [response] SingleItemResponse
                         */

                        /**
                         * Calls getIdentityBalance.
                         * @function getIdentityBalance
                         * @memberof org.dash.platform.dapi.v0.Platform
                         * @instance
                         * @param {org.dash.platform.dapi.v0.IGetSingleItemRequest} request GetSingleItemRequest message or plain object
                         * @param {org.dash.platform.dapi.v0.Platform.getIdentityBalanceCallback} callback Node-style callback called with the error, if any, and SingleItemResponse
                         * @returns {undefined}
                         * @variation 1
                         */
                        Object.defineProperty(Platform.prototype.getIdentityBalance = function getIdentityBalance(request, callback) {
                            return this.rpcCall(getIdentityBalance, $root.org.dash.platform.dapi.v0.GetSingleItemRequest, $root.org.dash.platform.dapi.v0.SingleItemResponse, request, callback);
                        }, "name", { value: "getIdentityBalance" });

                        /**
                         * Calls getIdentityBalance.
                         * @function getIdentityBalance
                         * @memberof org.dash.platform.dapi.v0.Platform
                         * @instance
                         * @param {org.dash.platform.dapi.v0.IGetSingleItemRequest} request GetSingleItemRequest message or plain object
                         * @returns {Promise<org.dash.platform.dapi.v0.SingleItemResponse>} Promise
                         * @variation 2
                         */

                        /**
                         * Callback as used by {@link org.dash.platform.dapi.v0.Platform#getIdentityBalanceAndRevision}.
                         * @memberof org.dash.platform.dapi.v0.Platform
                         * @typedef getIdentityBalanceAndRevisionCallback
                         * @type {function}
                         * @param {Error|null} error Error, if any
                         * @param {org.dash.platform.dapi.v0.SingleItemResponse} [response] SingleItemResponse
                         */

                        /**
                         * Calls getIdentityBalanceAndRevision.
                         * @function getIdentityBalanceAndRevision
                         * @memberof org.dash.platform.dapi.v0.Platform
                         * @instance
                         * @param {org.dash.platform.dapi.v0.IGetSingleItemRequest} request GetSingleItemRequest message or plain object
                         * @param {org.dash.platform.dapi.v0.Platform.getIdentityBalanceAndRevisionCallback} callback Node-style callback called with the error, if any, and SingleItemResponse
                         * @returns {undefined}
                         * @variation 1
                         */
                        Object.defineProperty(Platform.prototype.getIdentityBalanceAndRevision = function getIdentityBalanceAndRevision(request, callback) {
                            return this.rpcCall(getIdentityBalanceAndRevision, $root.org.dash.platform.dapi.v0.GetSingleItemRequest, $root.org.dash.platform.dapi.v0.SingleItemResponse, request, callback);
                        }, "name", { value: "getIdentityBalanceAndRevision" });

                        /**
                         * Calls getIdentityBalanceAndRevision.
                         * @function getIdentityBalanceAndRevision
                         * @memberof org.dash.platform.dapi.v0.Platform
                         * @instance
                         * @param {org.dash.platform.dapi.v0.IGetSingleItemRequest} request GetSingleItemRequest message or plain object
                         * @returns {Promise<org.dash.platform.dapi.v0.SingleItemResponse>} Promise
                         * @variation 2
                         */

                        /**
                         * Callback as used by {@link org.dash.platform.dapi.v0.Platform#getDataContract}.
                         * @memberof org.dash.platform.dapi.v0.Platform
                         * @typedef getDataContractCallback
                         * @type {function}
                         * @param {Error|null} error Error, if any
                         * @param {org.dash.platform.dapi.v0.SingleItemResponse} [response] SingleItemResponse
                         */

                        /**
                         * Calls getDataContract.
                         * @function getDataContract
                         * @memberof org.dash.platform.dapi.v0.Platform
                         * @instance
                         * @param {org.dash.platform.dapi.v0.IGetSingleItemRequest} request GetSingleItemRequest message or plain object
                         * @param {org.dash.platform.dapi.v0.Platform.getDataContractCallback} callback Node-style callback called with the error, if any, and SingleItemResponse
                         * @returns {undefined}
                         * @variation 1
                         */
                        Object.defineProperty(Platform.prototype.getDataContract = function getDataContract(request, callback) {
                            return this.rpcCall(getDataContract, $root.org.dash.platform.dapi.v0.GetSingleItemRequest, $root.org.dash.platform.dapi.v0.SingleItemResponse, request, callback);
                        }, "name", { value: "getDataContract" });

                        /**
                         * Calls getDataContract.
                         * @function getDataContract
                         * @memberof org.dash.platform.dapi.v0.Platform
                         * @instance
                         * @param {org.dash.platform.dapi.v0.IGetSingleItemRequest} request GetSingleItemRequest message or plain object
                         * @returns {Promise<org.dash.platform.dapi.v0.SingleItemResponse>} Promise
                         * @variation 2
                         */

                        /**
                         * Callback as used by {@link org.dash.platform.dapi.v0.Platform#getDocuments}.
                         * @memberof org.dash.platform.dapi.v0.Platform
                         * @typedef getDocumentsCallback
                         * @type {function}
                         * @param {Error|null} error Error, if any
                         * @param {org.dash.platform.dapi.v0.MultiItemResponse} [response] MultiItemResponse
                         */

                        /**
                         * Calls getDocuments.
                         * @function getDocuments
                         * @memberof org.dash.platform.dapi.v0.Platform
                         * @instance
                         * @param {org.dash.platform.dapi.v0.IGetDocumentsRequest} request GetDocumentsRequest message or plain object
                         * @param {org.dash.platform.dapi.v0.Platform.getDocumentsCallback} callback Node-style callback called with the error, if any, and MultiItemResponse
                         * @returns {undefined}
                         * @variation 1
                         */
                        Object.defineProperty(Platform.prototype.getDocuments = function getDocuments(request, callback) {
                            return this.rpcCall(getDocuments, $root.org.dash.platform.dapi.v0.GetDocumentsRequest, $root.org.dash.platform.dapi.v0.MultiItemResponse, request, callback);
                        }, "name", { value: "getDocuments" });

                        /**
                         * Calls getDocuments.
                         * @function getDocuments
                         * @memberof org.dash.platform.dapi.v0.Platform
                         * @instance
                         * @param {org.dash.platform.dapi.v0.IGetDocumentsRequest} request GetDocumentsRequest message or plain object
                         * @returns {Promise<org.dash.platform.dapi.v0.MultiItemResponse>} Promise
                         * @variation 2
                         */

                        /**
                         * Callback as used by {@link org.dash.platform.dapi.v0.Platform#getIdentitiesByPublicKeyHashes}.
                         * @memberof org.dash.platform.dapi.v0.Platform
                         * @typedef getIdentitiesByPublicKeyHashesCallback
                         * @type {function}
                         * @param {Error|null} error Error, if any
                         * @param {org.dash.platform.dapi.v0.MultiItemResponse} [response] MultiItemResponse
                         */

                        /**
                         * Calls getIdentitiesByPublicKeyHashes.
                         * @function getIdentitiesByPublicKeyHashes
                         * @memberof org.dash.platform.dapi.v0.Platform
                         * @instance
                         * @param {org.dash.platform.dapi.v0.IGetMultiItemRequest} request GetMultiItemRequest message or plain object
                         * @param {org.dash.platform.dapi.v0.Platform.getIdentitiesByPublicKeyHashesCallback} callback Node-style callback called with the error, if any, and MultiItemResponse
                         * @returns {undefined}
                         * @variation 1
                         */
                        Object.defineProperty(Platform.prototype.getIdentitiesByPublicKeyHashes = function getIdentitiesByPublicKeyHashes(request, callback) {
                            return this.rpcCall(getIdentitiesByPublicKeyHashes, $root.org.dash.platform.dapi.v0.GetMultiItemRequest, $root.org.dash.platform.dapi.v0.MultiItemResponse, request, callback);
                        }, "name", { value: "getIdentitiesByPublicKeyHashes" });

                        /**
                         * Calls getIdentitiesByPublicKeyHashes.
                         * @function getIdentitiesByPublicKeyHashes
                         * @memberof org.dash.platform.dapi.v0.Platform
                         * @instance
                         * @param {org.dash.platform.dapi.v0.IGetMultiItemRequest} request GetMultiItemRequest message or plain object
                         * @returns {Promise<org.dash.platform.dapi.v0.MultiItemResponse>} Promise
                         * @variation 2
                         */

                        /**
                         * Callback as used by {@link org.dash.platform.dapi.v0.Platform#waitForStateTransitionResult}.
                         * @memberof org.dash.platform.dapi.v0.Platform
                         * @typedef waitForStateTransitionResultCallback
                         * @type {function}
                         * @param {Error|null} error Error, if any
                         * @param {org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse} [response] WaitForStateTransitionResultResponse
                         */

                        /**
                         * Calls waitForStateTransitionResult.
                         * @function waitForStateTransitionResult
                         * @memberof org.dash.platform.dapi.v0.Platform
                         * @instance
                         * @param {org.dash.platform.dapi.v0.IWaitForStateTransitionResultRequest} request WaitForStateTransitionResultRequest message or plain object
                         * @param {org.dash.platform.dapi.v0.Platform.waitForStateTransitionResultCallback} callback Node-style callback called with the error, if any, and WaitForStateTransitionResultResponse
                         * @returns {undefined}
                         * @variation 1
                         */
                        Object.defineProperty(Platform.prototype.waitForStateTransitionResult = function waitForStateTransitionResult(request, callback) {
                            return this.rpcCall(waitForStateTransitionResult, $root.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest, $root.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse, request, callback);
                        }, "name", { value: "waitForStateTransitionResult" });

                        /**
                         * Calls waitForStateTransitionResult.
                         * @function waitForStateTransitionResult
                         * @memberof org.dash.platform.dapi.v0.Platform
                         * @instance
                         * @param {org.dash.platform.dapi.v0.IWaitForStateTransitionResultRequest} request WaitForStateTransitionResultRequest message or plain object
                         * @returns {Promise<org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse>} Promise
                         * @variation 2
                         */

                        /**
                         * Callback as used by {@link org.dash.platform.dapi.v0.Platform#getConsensusParams}.
                         * @memberof org.dash.platform.dapi.v0.Platform
                         * @typedef getConsensusParamsCallback
                         * @type {function}
                         * @param {Error|null} error Error, if any
                         * @param {org.dash.platform.dapi.v0.GetConsensusParamsResponse} [response] GetConsensusParamsResponse
                         */

                        /**
                         * Calls getConsensusParams.
                         * @function getConsensusParams
                         * @memberof org.dash.platform.dapi.v0.Platform
                         * @instance
                         * @param {org.dash.platform.dapi.v0.IGetConsensusParamsRequest} request GetConsensusParamsRequest message or plain object
                         * @param {org.dash.platform.dapi.v0.Platform.getConsensusParamsCallback} callback Node-style callback called with the error, if any, and GetConsensusParamsResponse
                         * @returns {undefined}
                         * @variation 1
                         */
                        Object.defineProperty(Platform.prototype.getConsensusParams = function getConsensusParams(request, callback) {
                            return this.rpcCall(getConsensusParams, $root.org.dash.platform.dapi.v0.GetConsensusParamsRequest, $root.org.dash.platform.dapi.v0.GetConsensusParamsResponse, request, callback);
                        }, "name", { value: "getConsensusParams" });

                        /**
                         * Calls getConsensusParams.
                         * @function getConsensusParams
                         * @memberof org.dash.platform.dapi.v0.Platform
                         * @instance
                         * @param {org.dash.platform.dapi.v0.IGetConsensusParamsRequest} request GetConsensusParamsRequest message or plain object
                         * @returns {Promise<org.dash.platform.dapi.v0.GetConsensusParamsResponse>} Promise
                         * @variation 2
                         */

                        return Platform;
                    })();

                    v0.ProvedResult = (function() {

                        /**
                         * Properties of a ProvedResult.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface IProvedResult
                         * @property {Uint8Array|null} [grovedbProof] ProvedResult grovedbProof
                         * @property {Uint8Array|null} [quorumHash] ProvedResult quorumHash
                         * @property {Uint8Array|null} [signature] ProvedResult signature
                         * @property {number|null} [round] ProvedResult round
                         */

                        /**
                         * Constructs a new ProvedResult.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a ProvedResult.
                         * @implements IProvedResult
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.IProvedResult=} [properties] Properties to set
                         */
                        function ProvedResult(properties) {
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * ProvedResult grovedbProof.
                         * @member {Uint8Array} grovedbProof
                         * @memberof org.dash.platform.dapi.v0.ProvedResult
                         * @instance
                         */
                        ProvedResult.prototype.grovedbProof = $util.newBuffer([]);

                        /**
                         * ProvedResult quorumHash.
                         * @member {Uint8Array} quorumHash
                         * @memberof org.dash.platform.dapi.v0.ProvedResult
                         * @instance
                         */
                        ProvedResult.prototype.quorumHash = $util.newBuffer([]);

                        /**
                         * ProvedResult signature.
                         * @member {Uint8Array} signature
                         * @memberof org.dash.platform.dapi.v0.ProvedResult
                         * @instance
                         */
                        ProvedResult.prototype.signature = $util.newBuffer([]);

                        /**
                         * ProvedResult round.
                         * @member {number} round
                         * @memberof org.dash.platform.dapi.v0.ProvedResult
                         * @instance
                         */
                        ProvedResult.prototype.round = 0;

                        /**
                         * Creates a new ProvedResult instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.ProvedResult
                         * @static
                         * @param {org.dash.platform.dapi.v0.IProvedResult=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.ProvedResult} ProvedResult instance
                         */
                        ProvedResult.create = function create(properties) {
                            return new ProvedResult(properties);
                        };

                        /**
                         * Encodes the specified ProvedResult message. Does not implicitly {@link org.dash.platform.dapi.v0.ProvedResult.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.ProvedResult
                         * @static
                         * @param {org.dash.platform.dapi.v0.IProvedResult} message ProvedResult message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        ProvedResult.encode = function encode(message, writer) {
                            if (!writer)
                                writer = $Writer.create();
                            if (message.grovedbProof != null && Object.hasOwnProperty.call(message, "grovedbProof"))
                                writer.uint32(/* id 1, wireType 2 =*/10).bytes(message.grovedbProof);
                            if (message.quorumHash != null && Object.hasOwnProperty.call(message, "quorumHash"))
                                writer.uint32(/* id 2, wireType 2 =*/18).bytes(message.quorumHash);
                            if (message.signature != null && Object.hasOwnProperty.call(message, "signature"))
                                writer.uint32(/* id 3, wireType 2 =*/26).bytes(message.signature);
                            if (message.round != null && Object.hasOwnProperty.call(message, "round"))
                                writer.uint32(/* id 4, wireType 0 =*/32).uint32(message.round);
                            return writer;
                        };

                        /**
                         * Encodes the specified ProvedResult message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.ProvedResult.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.ProvedResult
                         * @static
                         * @param {org.dash.platform.dapi.v0.IProvedResult} message ProvedResult message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        ProvedResult.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes a ProvedResult message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.ProvedResult
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.ProvedResult} ProvedResult
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        ProvedResult.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.ProvedResult();
                            while (reader.pos < end) {
                                var tag = reader.uint32();
                                switch (tag >>> 3) {
                                case 1:
                                    message.grovedbProof = reader.bytes();
                                    break;
                                case 2:
                                    message.quorumHash = reader.bytes();
                                    break;
                                case 3:
                                    message.signature = reader.bytes();
                                    break;
                                case 4:
                                    message.round = reader.uint32();
                                    break;
                                default:
                                    reader.skipType(tag & 7);
                                    break;
                                }
                            }
                            return message;
                        };

                        /**
                         * Decodes a ProvedResult message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.ProvedResult
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.ProvedResult} ProvedResult
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        ProvedResult.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies a ProvedResult message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.ProvedResult
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        ProvedResult.verify = function verify(message) {
                            if (typeof message !== "object" || message === null)
                                return "object expected";
                            if (message.grovedbProof != null && message.hasOwnProperty("grovedbProof"))
                                if (!(message.grovedbProof && typeof message.grovedbProof.length === "number" || $util.isString(message.grovedbProof)))
                                    return "grovedbProof: buffer expected";
                            if (message.quorumHash != null && message.hasOwnProperty("quorumHash"))
                                if (!(message.quorumHash && typeof message.quorumHash.length === "number" || $util.isString(message.quorumHash)))
                                    return "quorumHash: buffer expected";
                            if (message.signature != null && message.hasOwnProperty("signature"))
                                if (!(message.signature && typeof message.signature.length === "number" || $util.isString(message.signature)))
                                    return "signature: buffer expected";
                            if (message.round != null && message.hasOwnProperty("round"))
                                if (!$util.isInteger(message.round))
                                    return "round: integer expected";
                            return null;
                        };

                        /**
                         * Creates a ProvedResult message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.ProvedResult
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.ProvedResult} ProvedResult
                         */
                        ProvedResult.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.ProvedResult)
                                return object;
                            var message = new $root.org.dash.platform.dapi.v0.ProvedResult();
                            if (object.grovedbProof != null)
                                if (typeof object.grovedbProof === "string")
                                    $util.base64.decode(object.grovedbProof, message.grovedbProof = $util.newBuffer($util.base64.length(object.grovedbProof)), 0);
                                else if (object.grovedbProof.length >= 0)
                                    message.grovedbProof = object.grovedbProof;
                            if (object.quorumHash != null)
                                if (typeof object.quorumHash === "string")
                                    $util.base64.decode(object.quorumHash, message.quorumHash = $util.newBuffer($util.base64.length(object.quorumHash)), 0);
                                else if (object.quorumHash.length >= 0)
                                    message.quorumHash = object.quorumHash;
                            if (object.signature != null)
                                if (typeof object.signature === "string")
                                    $util.base64.decode(object.signature, message.signature = $util.newBuffer($util.base64.length(object.signature)), 0);
                                else if (object.signature.length >= 0)
                                    message.signature = object.signature;
                            if (object.round != null)
                                message.round = object.round >>> 0;
                            return message;
                        };

                        /**
                         * Creates a plain object from a ProvedResult message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.ProvedResult
                         * @static
                         * @param {org.dash.platform.dapi.v0.ProvedResult} message ProvedResult
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        ProvedResult.toObject = function toObject(message, options) {
                            if (!options)
                                options = {};
                            var object = {};
                            if (options.defaults) {
                                if (options.bytes === String)
                                    object.grovedbProof = "";
                                else {
                                    object.grovedbProof = [];
                                    if (options.bytes !== Array)
                                        object.grovedbProof = $util.newBuffer(object.grovedbProof);
                                }
                                if (options.bytes === String)
                                    object.quorumHash = "";
                                else {
                                    object.quorumHash = [];
                                    if (options.bytes !== Array)
                                        object.quorumHash = $util.newBuffer(object.quorumHash);
                                }
                                if (options.bytes === String)
                                    object.signature = "";
                                else {
                                    object.signature = [];
                                    if (options.bytes !== Array)
                                        object.signature = $util.newBuffer(object.signature);
                                }
                                object.round = 0;
                            }
                            if (message.grovedbProof != null && message.hasOwnProperty("grovedbProof"))
                                object.grovedbProof = options.bytes === String ? $util.base64.encode(message.grovedbProof, 0, message.grovedbProof.length) : options.bytes === Array ? Array.prototype.slice.call(message.grovedbProof) : message.grovedbProof;
                            if (message.quorumHash != null && message.hasOwnProperty("quorumHash"))
                                object.quorumHash = options.bytes === String ? $util.base64.encode(message.quorumHash, 0, message.quorumHash.length) : options.bytes === Array ? Array.prototype.slice.call(message.quorumHash) : message.quorumHash;
                            if (message.signature != null && message.hasOwnProperty("signature"))
                                object.signature = options.bytes === String ? $util.base64.encode(message.signature, 0, message.signature.length) : options.bytes === Array ? Array.prototype.slice.call(message.signature) : message.signature;
                            if (message.round != null && message.hasOwnProperty("round"))
                                object.round = message.round;
                            return object;
                        };

                        /**
                         * Converts this ProvedResult to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.ProvedResult
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        ProvedResult.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        return ProvedResult;
                    })();

                    v0.ResponseMetadata = (function() {

                        /**
                         * Properties of a ResponseMetadata.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface IResponseMetadata
                         * @property {number|Long|null} [height] ResponseMetadata height
                         * @property {number|null} [coreChainLockedHeight] ResponseMetadata coreChainLockedHeight
                         * @property {number|Long|null} [timeMs] ResponseMetadata timeMs
                         * @property {number|null} [protocolVersion] ResponseMetadata protocolVersion
                         */

                        /**
                         * Constructs a new ResponseMetadata.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a ResponseMetadata.
                         * @implements IResponseMetadata
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.IResponseMetadata=} [properties] Properties to set
                         */
                        function ResponseMetadata(properties) {
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * ResponseMetadata height.
                         * @member {number|Long} height
                         * @memberof org.dash.platform.dapi.v0.ResponseMetadata
                         * @instance
                         */
                        ResponseMetadata.prototype.height = $util.Long ? $util.Long.fromBits(0,0,false) : 0;

                        /**
                         * ResponseMetadata coreChainLockedHeight.
                         * @member {number} coreChainLockedHeight
                         * @memberof org.dash.platform.dapi.v0.ResponseMetadata
                         * @instance
                         */
                        ResponseMetadata.prototype.coreChainLockedHeight = 0;

                        /**
                         * ResponseMetadata timeMs.
                         * @member {number|Long} timeMs
                         * @memberof org.dash.platform.dapi.v0.ResponseMetadata
                         * @instance
                         */
                        ResponseMetadata.prototype.timeMs = $util.Long ? $util.Long.fromBits(0,0,true) : 0;

                        /**
                         * ResponseMetadata protocolVersion.
                         * @member {number} protocolVersion
                         * @memberof org.dash.platform.dapi.v0.ResponseMetadata
                         * @instance
                         */
                        ResponseMetadata.prototype.protocolVersion = 0;

                        /**
                         * Creates a new ResponseMetadata instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.ResponseMetadata
                         * @static
                         * @param {org.dash.platform.dapi.v0.IResponseMetadata=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.ResponseMetadata} ResponseMetadata instance
                         */
                        ResponseMetadata.create = function create(properties) {
                            return new ResponseMetadata(properties);
                        };

                        /**
                         * Encodes the specified ResponseMetadata message. Does not implicitly {@link org.dash.platform.dapi.v0.ResponseMetadata.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.ResponseMetadata
                         * @static
                         * @param {org.dash.platform.dapi.v0.IResponseMetadata} message ResponseMetadata message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        ResponseMetadata.encode = function encode(message, writer) {
                            if (!writer)
                                writer = $Writer.create();
                            if (message.height != null && Object.hasOwnProperty.call(message, "height"))
                                writer.uint32(/* id 1, wireType 0 =*/8).int64(message.height);
                            if (message.coreChainLockedHeight != null && Object.hasOwnProperty.call(message, "coreChainLockedHeight"))
                                writer.uint32(/* id 2, wireType 0 =*/16).uint32(message.coreChainLockedHeight);
                            if (message.timeMs != null && Object.hasOwnProperty.call(message, "timeMs"))
                                writer.uint32(/* id 3, wireType 0 =*/24).uint64(message.timeMs);
                            if (message.protocolVersion != null && Object.hasOwnProperty.call(message, "protocolVersion"))
                                writer.uint32(/* id 4, wireType 0 =*/32).uint32(message.protocolVersion);
                            return writer;
                        };

                        /**
                         * Encodes the specified ResponseMetadata message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.ResponseMetadata.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.ResponseMetadata
                         * @static
                         * @param {org.dash.platform.dapi.v0.IResponseMetadata} message ResponseMetadata message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        ResponseMetadata.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes a ResponseMetadata message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.ResponseMetadata
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.ResponseMetadata} ResponseMetadata
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        ResponseMetadata.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.ResponseMetadata();
                            while (reader.pos < end) {
                                var tag = reader.uint32();
                                switch (tag >>> 3) {
                                case 1:
                                    message.height = reader.int64();
                                    break;
                                case 2:
                                    message.coreChainLockedHeight = reader.uint32();
                                    break;
                                case 3:
                                    message.timeMs = reader.uint64();
                                    break;
                                case 4:
                                    message.protocolVersion = reader.uint32();
                                    break;
                                default:
                                    reader.skipType(tag & 7);
                                    break;
                                }
                            }
                            return message;
                        };

                        /**
                         * Decodes a ResponseMetadata message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.ResponseMetadata
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.ResponseMetadata} ResponseMetadata
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        ResponseMetadata.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies a ResponseMetadata message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.ResponseMetadata
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        ResponseMetadata.verify = function verify(message) {
                            if (typeof message !== "object" || message === null)
                                return "object expected";
                            if (message.height != null && message.hasOwnProperty("height"))
                                if (!$util.isInteger(message.height) && !(message.height && $util.isInteger(message.height.low) && $util.isInteger(message.height.high)))
                                    return "height: integer|Long expected";
                            if (message.coreChainLockedHeight != null && message.hasOwnProperty("coreChainLockedHeight"))
                                if (!$util.isInteger(message.coreChainLockedHeight))
                                    return "coreChainLockedHeight: integer expected";
                            if (message.timeMs != null && message.hasOwnProperty("timeMs"))
                                if (!$util.isInteger(message.timeMs) && !(message.timeMs && $util.isInteger(message.timeMs.low) && $util.isInteger(message.timeMs.high)))
                                    return "timeMs: integer|Long expected";
                            if (message.protocolVersion != null && message.hasOwnProperty("protocolVersion"))
                                if (!$util.isInteger(message.protocolVersion))
                                    return "protocolVersion: integer expected";
                            return null;
                        };

                        /**
                         * Creates a ResponseMetadata message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.ResponseMetadata
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.ResponseMetadata} ResponseMetadata
                         */
                        ResponseMetadata.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.ResponseMetadata)
                                return object;
                            var message = new $root.org.dash.platform.dapi.v0.ResponseMetadata();
                            if (object.height != null)
                                if ($util.Long)
                                    (message.height = $util.Long.fromValue(object.height)).unsigned = false;
                                else if (typeof object.height === "string")
                                    message.height = parseInt(object.height, 10);
                                else if (typeof object.height === "number")
                                    message.height = object.height;
                                else if (typeof object.height === "object")
                                    message.height = new $util.LongBits(object.height.low >>> 0, object.height.high >>> 0).toNumber();
                            if (object.coreChainLockedHeight != null)
                                message.coreChainLockedHeight = object.coreChainLockedHeight >>> 0;
                            if (object.timeMs != null)
                                if ($util.Long)
                                    (message.timeMs = $util.Long.fromValue(object.timeMs)).unsigned = true;
                                else if (typeof object.timeMs === "string")
                                    message.timeMs = parseInt(object.timeMs, 10);
                                else if (typeof object.timeMs === "number")
                                    message.timeMs = object.timeMs;
                                else if (typeof object.timeMs === "object")
                                    message.timeMs = new $util.LongBits(object.timeMs.low >>> 0, object.timeMs.high >>> 0).toNumber(true);
                            if (object.protocolVersion != null)
                                message.protocolVersion = object.protocolVersion >>> 0;
                            return message;
                        };

                        /**
                         * Creates a plain object from a ResponseMetadata message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.ResponseMetadata
                         * @static
                         * @param {org.dash.platform.dapi.v0.ResponseMetadata} message ResponseMetadata
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        ResponseMetadata.toObject = function toObject(message, options) {
                            if (!options)
                                options = {};
                            var object = {};
                            if (options.defaults) {
                                if ($util.Long) {
                                    var long = new $util.Long(0, 0, false);
                                    object.height = options.longs === String ? long.toString() : options.longs === Number ? long.toNumber() : long;
                                } else
                                    object.height = options.longs === String ? "0" : 0;
                                object.coreChainLockedHeight = 0;
                                if ($util.Long) {
                                    var long = new $util.Long(0, 0, true);
                                    object.timeMs = options.longs === String ? long.toString() : options.longs === Number ? long.toNumber() : long;
                                } else
                                    object.timeMs = options.longs === String ? "0" : 0;
                                object.protocolVersion = 0;
                            }
                            if (message.height != null && message.hasOwnProperty("height"))
                                if (typeof message.height === "number")
                                    object.height = options.longs === String ? String(message.height) : message.height;
                                else
                                    object.height = options.longs === String ? $util.Long.prototype.toString.call(message.height) : options.longs === Number ? new $util.LongBits(message.height.low >>> 0, message.height.high >>> 0).toNumber() : message.height;
                            if (message.coreChainLockedHeight != null && message.hasOwnProperty("coreChainLockedHeight"))
                                object.coreChainLockedHeight = message.coreChainLockedHeight;
                            if (message.timeMs != null && message.hasOwnProperty("timeMs"))
                                if (typeof message.timeMs === "number")
                                    object.timeMs = options.longs === String ? String(message.timeMs) : message.timeMs;
                                else
                                    object.timeMs = options.longs === String ? $util.Long.prototype.toString.call(message.timeMs) : options.longs === Number ? new $util.LongBits(message.timeMs.low >>> 0, message.timeMs.high >>> 0).toNumber(true) : message.timeMs;
                            if (message.protocolVersion != null && message.hasOwnProperty("protocolVersion"))
                                object.protocolVersion = message.protocolVersion;
                            return object;
                        };

                        /**
                         * Converts this ResponseMetadata to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.ResponseMetadata
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        ResponseMetadata.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        return ResponseMetadata;
                    })();

                    v0.StateTransitionBroadcastError = (function() {

                        /**
                         * Properties of a StateTransitionBroadcastError.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface IStateTransitionBroadcastError
                         * @property {number|null} [code] StateTransitionBroadcastError code
                         * @property {string|null} [message] StateTransitionBroadcastError message
                         * @property {Uint8Array|null} [data] StateTransitionBroadcastError data
                         */

                        /**
                         * Constructs a new StateTransitionBroadcastError.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a StateTransitionBroadcastError.
                         * @implements IStateTransitionBroadcastError
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.IStateTransitionBroadcastError=} [properties] Properties to set
                         */
                        function StateTransitionBroadcastError(properties) {
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * StateTransitionBroadcastError code.
                         * @member {number} code
                         * @memberof org.dash.platform.dapi.v0.StateTransitionBroadcastError
                         * @instance
                         */
                        StateTransitionBroadcastError.prototype.code = 0;

                        /**
                         * StateTransitionBroadcastError message.
                         * @member {string} message
                         * @memberof org.dash.platform.dapi.v0.StateTransitionBroadcastError
                         * @instance
                         */
                        StateTransitionBroadcastError.prototype.message = "";

                        /**
                         * StateTransitionBroadcastError data.
                         * @member {Uint8Array} data
                         * @memberof org.dash.platform.dapi.v0.StateTransitionBroadcastError
                         * @instance
                         */
                        StateTransitionBroadcastError.prototype.data = $util.newBuffer([]);

                        /**
                         * Creates a new StateTransitionBroadcastError instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.StateTransitionBroadcastError
                         * @static
                         * @param {org.dash.platform.dapi.v0.IStateTransitionBroadcastError=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.StateTransitionBroadcastError} StateTransitionBroadcastError instance
                         */
                        StateTransitionBroadcastError.create = function create(properties) {
                            return new StateTransitionBroadcastError(properties);
                        };

                        /**
                         * Encodes the specified StateTransitionBroadcastError message. Does not implicitly {@link org.dash.platform.dapi.v0.StateTransitionBroadcastError.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.StateTransitionBroadcastError
                         * @static
                         * @param {org.dash.platform.dapi.v0.IStateTransitionBroadcastError} message StateTransitionBroadcastError message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        StateTransitionBroadcastError.encode = function encode(message, writer) {
                            if (!writer)
                                writer = $Writer.create();
                            if (message.code != null && Object.hasOwnProperty.call(message, "code"))
                                writer.uint32(/* id 1, wireType 0 =*/8).uint32(message.code);
                            if (message.message != null && Object.hasOwnProperty.call(message, "message"))
                                writer.uint32(/* id 2, wireType 2 =*/18).string(message.message);
                            if (message.data != null && Object.hasOwnProperty.call(message, "data"))
                                writer.uint32(/* id 3, wireType 2 =*/26).bytes(message.data);
                            return writer;
                        };

                        /**
                         * Encodes the specified StateTransitionBroadcastError message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.StateTransitionBroadcastError.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.StateTransitionBroadcastError
                         * @static
                         * @param {org.dash.platform.dapi.v0.IStateTransitionBroadcastError} message StateTransitionBroadcastError message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        StateTransitionBroadcastError.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes a StateTransitionBroadcastError message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.StateTransitionBroadcastError
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.StateTransitionBroadcastError} StateTransitionBroadcastError
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        StateTransitionBroadcastError.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.StateTransitionBroadcastError();
                            while (reader.pos < end) {
                                var tag = reader.uint32();
                                switch (tag >>> 3) {
                                case 1:
                                    message.code = reader.uint32();
                                    break;
                                case 2:
                                    message.message = reader.string();
                                    break;
                                case 3:
                                    message.data = reader.bytes();
                                    break;
                                default:
                                    reader.skipType(tag & 7);
                                    break;
                                }
                            }
                            return message;
                        };

                        /**
                         * Decodes a StateTransitionBroadcastError message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.StateTransitionBroadcastError
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.StateTransitionBroadcastError} StateTransitionBroadcastError
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        StateTransitionBroadcastError.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies a StateTransitionBroadcastError message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.StateTransitionBroadcastError
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        StateTransitionBroadcastError.verify = function verify(message) {
                            if (typeof message !== "object" || message === null)
                                return "object expected";
                            if (message.code != null && message.hasOwnProperty("code"))
                                if (!$util.isInteger(message.code))
                                    return "code: integer expected";
                            if (message.message != null && message.hasOwnProperty("message"))
                                if (!$util.isString(message.message))
                                    return "message: string expected";
                            if (message.data != null && message.hasOwnProperty("data"))
                                if (!(message.data && typeof message.data.length === "number" || $util.isString(message.data)))
                                    return "data: buffer expected";
                            return null;
                        };

                        /**
                         * Creates a StateTransitionBroadcastError message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.StateTransitionBroadcastError
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.StateTransitionBroadcastError} StateTransitionBroadcastError
                         */
                        StateTransitionBroadcastError.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.StateTransitionBroadcastError)
                                return object;
                            var message = new $root.org.dash.platform.dapi.v0.StateTransitionBroadcastError();
                            if (object.code != null)
                                message.code = object.code >>> 0;
                            if (object.message != null)
                                message.message = String(object.message);
                            if (object.data != null)
                                if (typeof object.data === "string")
                                    $util.base64.decode(object.data, message.data = $util.newBuffer($util.base64.length(object.data)), 0);
                                else if (object.data.length >= 0)
                                    message.data = object.data;
                            return message;
                        };

                        /**
                         * Creates a plain object from a StateTransitionBroadcastError message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.StateTransitionBroadcastError
                         * @static
                         * @param {org.dash.platform.dapi.v0.StateTransitionBroadcastError} message StateTransitionBroadcastError
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        StateTransitionBroadcastError.toObject = function toObject(message, options) {
                            if (!options)
                                options = {};
                            var object = {};
                            if (options.defaults) {
                                object.code = 0;
                                object.message = "";
                                if (options.bytes === String)
                                    object.data = "";
                                else {
                                    object.data = [];
                                    if (options.bytes !== Array)
                                        object.data = $util.newBuffer(object.data);
                                }
                            }
                            if (message.code != null && message.hasOwnProperty("code"))
                                object.code = message.code;
                            if (message.message != null && message.hasOwnProperty("message"))
                                object.message = message.message;
                            if (message.data != null && message.hasOwnProperty("data"))
                                object.data = options.bytes === String ? $util.base64.encode(message.data, 0, message.data.length) : options.bytes === Array ? Array.prototype.slice.call(message.data) : message.data;
                            return object;
                        };

                        /**
                         * Converts this StateTransitionBroadcastError to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.StateTransitionBroadcastError
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        StateTransitionBroadcastError.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        return StateTransitionBroadcastError;
                    })();

                    v0.BroadcastStateTransitionRequest = (function() {

                        /**
                         * Properties of a BroadcastStateTransitionRequest.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface IBroadcastStateTransitionRequest
                         * @property {Uint8Array|null} [stateTransition] BroadcastStateTransitionRequest stateTransition
                         */

                        /**
                         * Constructs a new BroadcastStateTransitionRequest.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a BroadcastStateTransitionRequest.
                         * @implements IBroadcastStateTransitionRequest
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.IBroadcastStateTransitionRequest=} [properties] Properties to set
                         */
                        function BroadcastStateTransitionRequest(properties) {
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * BroadcastStateTransitionRequest stateTransition.
                         * @member {Uint8Array} stateTransition
                         * @memberof org.dash.platform.dapi.v0.BroadcastStateTransitionRequest
                         * @instance
                         */
                        BroadcastStateTransitionRequest.prototype.stateTransition = $util.newBuffer([]);

                        /**
                         * Creates a new BroadcastStateTransitionRequest instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.BroadcastStateTransitionRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.IBroadcastStateTransitionRequest=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.BroadcastStateTransitionRequest} BroadcastStateTransitionRequest instance
                         */
                        BroadcastStateTransitionRequest.create = function create(properties) {
                            return new BroadcastStateTransitionRequest(properties);
                        };

                        /**
                         * Encodes the specified BroadcastStateTransitionRequest message. Does not implicitly {@link org.dash.platform.dapi.v0.BroadcastStateTransitionRequest.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.BroadcastStateTransitionRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.IBroadcastStateTransitionRequest} message BroadcastStateTransitionRequest message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        BroadcastStateTransitionRequest.encode = function encode(message, writer) {
                            if (!writer)
                                writer = $Writer.create();
                            if (message.stateTransition != null && Object.hasOwnProperty.call(message, "stateTransition"))
                                writer.uint32(/* id 1, wireType 2 =*/10).bytes(message.stateTransition);
                            return writer;
                        };

                        /**
                         * Encodes the specified BroadcastStateTransitionRequest message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.BroadcastStateTransitionRequest.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.BroadcastStateTransitionRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.IBroadcastStateTransitionRequest} message BroadcastStateTransitionRequest message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        BroadcastStateTransitionRequest.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes a BroadcastStateTransitionRequest message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.BroadcastStateTransitionRequest
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.BroadcastStateTransitionRequest} BroadcastStateTransitionRequest
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        BroadcastStateTransitionRequest.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.BroadcastStateTransitionRequest();
                            while (reader.pos < end) {
                                var tag = reader.uint32();
                                switch (tag >>> 3) {
                                case 1:
                                    message.stateTransition = reader.bytes();
                                    break;
                                default:
                                    reader.skipType(tag & 7);
                                    break;
                                }
                            }
                            return message;
                        };

                        /**
                         * Decodes a BroadcastStateTransitionRequest message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.BroadcastStateTransitionRequest
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.BroadcastStateTransitionRequest} BroadcastStateTransitionRequest
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        BroadcastStateTransitionRequest.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies a BroadcastStateTransitionRequest message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.BroadcastStateTransitionRequest
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        BroadcastStateTransitionRequest.verify = function verify(message) {
                            if (typeof message !== "object" || message === null)
                                return "object expected";
                            if (message.stateTransition != null && message.hasOwnProperty("stateTransition"))
                                if (!(message.stateTransition && typeof message.stateTransition.length === "number" || $util.isString(message.stateTransition)))
                                    return "stateTransition: buffer expected";
                            return null;
                        };

                        /**
                         * Creates a BroadcastStateTransitionRequest message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.BroadcastStateTransitionRequest
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.BroadcastStateTransitionRequest} BroadcastStateTransitionRequest
                         */
                        BroadcastStateTransitionRequest.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.BroadcastStateTransitionRequest)
                                return object;
                            var message = new $root.org.dash.platform.dapi.v0.BroadcastStateTransitionRequest();
                            if (object.stateTransition != null)
                                if (typeof object.stateTransition === "string")
                                    $util.base64.decode(object.stateTransition, message.stateTransition = $util.newBuffer($util.base64.length(object.stateTransition)), 0);
                                else if (object.stateTransition.length >= 0)
                                    message.stateTransition = object.stateTransition;
                            return message;
                        };

                        /**
                         * Creates a plain object from a BroadcastStateTransitionRequest message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.BroadcastStateTransitionRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.BroadcastStateTransitionRequest} message BroadcastStateTransitionRequest
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        BroadcastStateTransitionRequest.toObject = function toObject(message, options) {
                            if (!options)
                                options = {};
                            var object = {};
                            if (options.defaults)
                                if (options.bytes === String)
                                    object.stateTransition = "";
                                else {
                                    object.stateTransition = [];
                                    if (options.bytes !== Array)
                                        object.stateTransition = $util.newBuffer(object.stateTransition);
                                }
                            if (message.stateTransition != null && message.hasOwnProperty("stateTransition"))
                                object.stateTransition = options.bytes === String ? $util.base64.encode(message.stateTransition, 0, message.stateTransition.length) : options.bytes === Array ? Array.prototype.slice.call(message.stateTransition) : message.stateTransition;
                            return object;
                        };

                        /**
                         * Converts this BroadcastStateTransitionRequest to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.BroadcastStateTransitionRequest
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        BroadcastStateTransitionRequest.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        return BroadcastStateTransitionRequest;
                    })();

                    v0.BroadcastStateTransitionResponse = (function() {

                        /**
                         * Properties of a BroadcastStateTransitionResponse.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface IBroadcastStateTransitionResponse
                         */

                        /**
                         * Constructs a new BroadcastStateTransitionResponse.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a BroadcastStateTransitionResponse.
                         * @implements IBroadcastStateTransitionResponse
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.IBroadcastStateTransitionResponse=} [properties] Properties to set
                         */
                        function BroadcastStateTransitionResponse(properties) {
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * Creates a new BroadcastStateTransitionResponse instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.BroadcastStateTransitionResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.IBroadcastStateTransitionResponse=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.BroadcastStateTransitionResponse} BroadcastStateTransitionResponse instance
                         */
                        BroadcastStateTransitionResponse.create = function create(properties) {
                            return new BroadcastStateTransitionResponse(properties);
                        };

                        /**
                         * Encodes the specified BroadcastStateTransitionResponse message. Does not implicitly {@link org.dash.platform.dapi.v0.BroadcastStateTransitionResponse.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.BroadcastStateTransitionResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.IBroadcastStateTransitionResponse} message BroadcastStateTransitionResponse message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        BroadcastStateTransitionResponse.encode = function encode(message, writer) {
                            if (!writer)
                                writer = $Writer.create();
                            return writer;
                        };

                        /**
                         * Encodes the specified BroadcastStateTransitionResponse message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.BroadcastStateTransitionResponse.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.BroadcastStateTransitionResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.IBroadcastStateTransitionResponse} message BroadcastStateTransitionResponse message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        BroadcastStateTransitionResponse.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes a BroadcastStateTransitionResponse message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.BroadcastStateTransitionResponse
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.BroadcastStateTransitionResponse} BroadcastStateTransitionResponse
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        BroadcastStateTransitionResponse.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.BroadcastStateTransitionResponse();
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
                         * Decodes a BroadcastStateTransitionResponse message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.BroadcastStateTransitionResponse
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.BroadcastStateTransitionResponse} BroadcastStateTransitionResponse
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        BroadcastStateTransitionResponse.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies a BroadcastStateTransitionResponse message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.BroadcastStateTransitionResponse
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        BroadcastStateTransitionResponse.verify = function verify(message) {
                            if (typeof message !== "object" || message === null)
                                return "object expected";
                            return null;
                        };

                        /**
                         * Creates a BroadcastStateTransitionResponse message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.BroadcastStateTransitionResponse
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.BroadcastStateTransitionResponse} BroadcastStateTransitionResponse
                         */
                        BroadcastStateTransitionResponse.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.BroadcastStateTransitionResponse)
                                return object;
                            return new $root.org.dash.platform.dapi.v0.BroadcastStateTransitionResponse();
                        };

                        /**
                         * Creates a plain object from a BroadcastStateTransitionResponse message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.BroadcastStateTransitionResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.BroadcastStateTransitionResponse} message BroadcastStateTransitionResponse
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        BroadcastStateTransitionResponse.toObject = function toObject() {
                            return {};
                        };

                        /**
                         * Converts this BroadcastStateTransitionResponse to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.BroadcastStateTransitionResponse
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        BroadcastStateTransitionResponse.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        return BroadcastStateTransitionResponse;
                    })();

                    v0.SingleItemResponse = (function() {

                        /**
                         * Properties of a SingleItemResponse.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface ISingleItemResponse
                         * @property {Uint8Array|null} [nonProvedResult] SingleItemResponse nonProvedResult
                         * @property {org.dash.platform.dapi.v0.IProvedResult|null} [provedResult] SingleItemResponse provedResult
                         * @property {org.dash.platform.dapi.v0.IResponseMetadata|null} [metadata] SingleItemResponse metadata
                         */

                        /**
                         * Constructs a new SingleItemResponse.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a SingleItemResponse.
                         * @implements ISingleItemResponse
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.ISingleItemResponse=} [properties] Properties to set
                         */
                        function SingleItemResponse(properties) {
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * SingleItemResponse nonProvedResult.
                         * @member {Uint8Array} nonProvedResult
                         * @memberof org.dash.platform.dapi.v0.SingleItemResponse
                         * @instance
                         */
                        SingleItemResponse.prototype.nonProvedResult = $util.newBuffer([]);

                        /**
                         * SingleItemResponse provedResult.
                         * @member {org.dash.platform.dapi.v0.IProvedResult|null|undefined} provedResult
                         * @memberof org.dash.platform.dapi.v0.SingleItemResponse
                         * @instance
                         */
                        SingleItemResponse.prototype.provedResult = null;

                        /**
                         * SingleItemResponse metadata.
                         * @member {org.dash.platform.dapi.v0.IResponseMetadata|null|undefined} metadata
                         * @memberof org.dash.platform.dapi.v0.SingleItemResponse
                         * @instance
                         */
                        SingleItemResponse.prototype.metadata = null;

                        // OneOf field names bound to virtual getters and setters
                        var $oneOfFields;

                        /**
                         * SingleItemResponse result.
                         * @member {"nonProvedResult"|"provedResult"|undefined} result
                         * @memberof org.dash.platform.dapi.v0.SingleItemResponse
                         * @instance
                         */
                        Object.defineProperty(SingleItemResponse.prototype, "result", {
                            get: $util.oneOfGetter($oneOfFields = ["nonProvedResult", "provedResult"]),
                            set: $util.oneOfSetter($oneOfFields)
                        });

                        /**
                         * Creates a new SingleItemResponse instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.SingleItemResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.ISingleItemResponse=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.SingleItemResponse} SingleItemResponse instance
                         */
                        SingleItemResponse.create = function create(properties) {
                            return new SingleItemResponse(properties);
                        };

                        /**
                         * Encodes the specified SingleItemResponse message. Does not implicitly {@link org.dash.platform.dapi.v0.SingleItemResponse.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.SingleItemResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.ISingleItemResponse} message SingleItemResponse message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        SingleItemResponse.encode = function encode(message, writer) {
                            if (!writer)
                                writer = $Writer.create();
                            if (message.nonProvedResult != null && Object.hasOwnProperty.call(message, "nonProvedResult"))
                                writer.uint32(/* id 1, wireType 2 =*/10).bytes(message.nonProvedResult);
                            if (message.provedResult != null && Object.hasOwnProperty.call(message, "provedResult"))
                                $root.org.dash.platform.dapi.v0.ProvedResult.encode(message.provedResult, writer.uint32(/* id 2, wireType 2 =*/18).fork()).ldelim();
                            if (message.metadata != null && Object.hasOwnProperty.call(message, "metadata"))
                                $root.org.dash.platform.dapi.v0.ResponseMetadata.encode(message.metadata, writer.uint32(/* id 3, wireType 2 =*/26).fork()).ldelim();
                            return writer;
                        };

                        /**
                         * Encodes the specified SingleItemResponse message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.SingleItemResponse.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.SingleItemResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.ISingleItemResponse} message SingleItemResponse message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        SingleItemResponse.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes a SingleItemResponse message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.SingleItemResponse
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.SingleItemResponse} SingleItemResponse
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        SingleItemResponse.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.SingleItemResponse();
                            while (reader.pos < end) {
                                var tag = reader.uint32();
                                switch (tag >>> 3) {
                                case 1:
                                    message.nonProvedResult = reader.bytes();
                                    break;
                                case 2:
                                    message.provedResult = $root.org.dash.platform.dapi.v0.ProvedResult.decode(reader, reader.uint32());
                                    break;
                                case 3:
                                    message.metadata = $root.org.dash.platform.dapi.v0.ResponseMetadata.decode(reader, reader.uint32());
                                    break;
                                default:
                                    reader.skipType(tag & 7);
                                    break;
                                }
                            }
                            return message;
                        };

                        /**
                         * Decodes a SingleItemResponse message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.SingleItemResponse
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.SingleItemResponse} SingleItemResponse
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        SingleItemResponse.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies a SingleItemResponse message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.SingleItemResponse
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        SingleItemResponse.verify = function verify(message) {
                            if (typeof message !== "object" || message === null)
                                return "object expected";
                            var properties = {};
                            if (message.nonProvedResult != null && message.hasOwnProperty("nonProvedResult")) {
                                properties.result = 1;
                                if (!(message.nonProvedResult && typeof message.nonProvedResult.length === "number" || $util.isString(message.nonProvedResult)))
                                    return "nonProvedResult: buffer expected";
                            }
                            if (message.provedResult != null && message.hasOwnProperty("provedResult")) {
                                if (properties.result === 1)
                                    return "result: multiple values";
                                properties.result = 1;
                                {
                                    var error = $root.org.dash.platform.dapi.v0.ProvedResult.verify(message.provedResult);
                                    if (error)
                                        return "provedResult." + error;
                                }
                            }
                            if (message.metadata != null && message.hasOwnProperty("metadata")) {
                                var error = $root.org.dash.platform.dapi.v0.ResponseMetadata.verify(message.metadata);
                                if (error)
                                    return "metadata." + error;
                            }
                            return null;
                        };

                        /**
                         * Creates a SingleItemResponse message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.SingleItemResponse
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.SingleItemResponse} SingleItemResponse
                         */
                        SingleItemResponse.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.SingleItemResponse)
                                return object;
                            var message = new $root.org.dash.platform.dapi.v0.SingleItemResponse();
                            if (object.nonProvedResult != null)
                                if (typeof object.nonProvedResult === "string")
                                    $util.base64.decode(object.nonProvedResult, message.nonProvedResult = $util.newBuffer($util.base64.length(object.nonProvedResult)), 0);
                                else if (object.nonProvedResult.length >= 0)
                                    message.nonProvedResult = object.nonProvedResult;
                            if (object.provedResult != null) {
                                if (typeof object.provedResult !== "object")
                                    throw TypeError(".org.dash.platform.dapi.v0.SingleItemResponse.provedResult: object expected");
                                message.provedResult = $root.org.dash.platform.dapi.v0.ProvedResult.fromObject(object.provedResult);
                            }
                            if (object.metadata != null) {
                                if (typeof object.metadata !== "object")
                                    throw TypeError(".org.dash.platform.dapi.v0.SingleItemResponse.metadata: object expected");
                                message.metadata = $root.org.dash.platform.dapi.v0.ResponseMetadata.fromObject(object.metadata);
                            }
                            return message;
                        };

                        /**
                         * Creates a plain object from a SingleItemResponse message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.SingleItemResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.SingleItemResponse} message SingleItemResponse
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        SingleItemResponse.toObject = function toObject(message, options) {
                            if (!options)
                                options = {};
                            var object = {};
                            if (options.defaults)
                                object.metadata = null;
                            if (message.nonProvedResult != null && message.hasOwnProperty("nonProvedResult")) {
                                object.nonProvedResult = options.bytes === String ? $util.base64.encode(message.nonProvedResult, 0, message.nonProvedResult.length) : options.bytes === Array ? Array.prototype.slice.call(message.nonProvedResult) : message.nonProvedResult;
                                if (options.oneofs)
                                    object.result = "nonProvedResult";
                            }
                            if (message.provedResult != null && message.hasOwnProperty("provedResult")) {
                                object.provedResult = $root.org.dash.platform.dapi.v0.ProvedResult.toObject(message.provedResult, options);
                                if (options.oneofs)
                                    object.result = "provedResult";
                            }
                            if (message.metadata != null && message.hasOwnProperty("metadata"))
                                object.metadata = $root.org.dash.platform.dapi.v0.ResponseMetadata.toObject(message.metadata, options);
                            return object;
                        };

                        /**
                         * Converts this SingleItemResponse to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.SingleItemResponse
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        SingleItemResponse.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        return SingleItemResponse;
                    })();

                    v0.ResultList = (function() {

                        /**
                         * Properties of a ResultList.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface IResultList
                         * @property {Array.<Uint8Array>|null} [items] ResultList items
                         */

                        /**
                         * Constructs a new ResultList.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a ResultList.
                         * @implements IResultList
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.IResultList=} [properties] Properties to set
                         */
                        function ResultList(properties) {
                            this.items = [];
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * ResultList items.
                         * @member {Array.<Uint8Array>} items
                         * @memberof org.dash.platform.dapi.v0.ResultList
                         * @instance
                         */
                        ResultList.prototype.items = $util.emptyArray;

                        /**
                         * Creates a new ResultList instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.ResultList
                         * @static
                         * @param {org.dash.platform.dapi.v0.IResultList=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.ResultList} ResultList instance
                         */
                        ResultList.create = function create(properties) {
                            return new ResultList(properties);
                        };

                        /**
                         * Encodes the specified ResultList message. Does not implicitly {@link org.dash.platform.dapi.v0.ResultList.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.ResultList
                         * @static
                         * @param {org.dash.platform.dapi.v0.IResultList} message ResultList message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        ResultList.encode = function encode(message, writer) {
                            if (!writer)
                                writer = $Writer.create();
                            if (message.items != null && message.items.length)
                                for (var i = 0; i < message.items.length; ++i)
                                    writer.uint32(/* id 1, wireType 2 =*/10).bytes(message.items[i]);
                            return writer;
                        };

                        /**
                         * Encodes the specified ResultList message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.ResultList.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.ResultList
                         * @static
                         * @param {org.dash.platform.dapi.v0.IResultList} message ResultList message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        ResultList.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes a ResultList message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.ResultList
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.ResultList} ResultList
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        ResultList.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.ResultList();
                            while (reader.pos < end) {
                                var tag = reader.uint32();
                                switch (tag >>> 3) {
                                case 1:
                                    if (!(message.items && message.items.length))
                                        message.items = [];
                                    message.items.push(reader.bytes());
                                    break;
                                default:
                                    reader.skipType(tag & 7);
                                    break;
                                }
                            }
                            return message;
                        };

                        /**
                         * Decodes a ResultList message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.ResultList
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.ResultList} ResultList
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        ResultList.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies a ResultList message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.ResultList
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        ResultList.verify = function verify(message) {
                            if (typeof message !== "object" || message === null)
                                return "object expected";
                            if (message.items != null && message.hasOwnProperty("items")) {
                                if (!Array.isArray(message.items))
                                    return "items: array expected";
                                for (var i = 0; i < message.items.length; ++i)
                                    if (!(message.items[i] && typeof message.items[i].length === "number" || $util.isString(message.items[i])))
                                        return "items: buffer[] expected";
                            }
                            return null;
                        };

                        /**
                         * Creates a ResultList message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.ResultList
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.ResultList} ResultList
                         */
                        ResultList.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.ResultList)
                                return object;
                            var message = new $root.org.dash.platform.dapi.v0.ResultList();
                            if (object.items) {
                                if (!Array.isArray(object.items))
                                    throw TypeError(".org.dash.platform.dapi.v0.ResultList.items: array expected");
                                message.items = [];
                                for (var i = 0; i < object.items.length; ++i)
                                    if (typeof object.items[i] === "string")
                                        $util.base64.decode(object.items[i], message.items[i] = $util.newBuffer($util.base64.length(object.items[i])), 0);
                                    else if (object.items[i].length >= 0)
                                        message.items[i] = object.items[i];
                            }
                            return message;
                        };

                        /**
                         * Creates a plain object from a ResultList message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.ResultList
                         * @static
                         * @param {org.dash.platform.dapi.v0.ResultList} message ResultList
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        ResultList.toObject = function toObject(message, options) {
                            if (!options)
                                options = {};
                            var object = {};
                            if (options.arrays || options.defaults)
                                object.items = [];
                            if (message.items && message.items.length) {
                                object.items = [];
                                for (var j = 0; j < message.items.length; ++j)
                                    object.items[j] = options.bytes === String ? $util.base64.encode(message.items[j], 0, message.items[j].length) : options.bytes === Array ? Array.prototype.slice.call(message.items[j]) : message.items[j];
                            }
                            return object;
                        };

                        /**
                         * Converts this ResultList to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.ResultList
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        ResultList.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        return ResultList;
                    })();

                    v0.MultiItemResponse = (function() {

                        /**
                         * Properties of a MultiItemResponse.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface IMultiItemResponse
                         * @property {org.dash.platform.dapi.v0.IResultList|null} [nonProvedResults] MultiItemResponse nonProvedResults
                         * @property {org.dash.platform.dapi.v0.IProvedResult|null} [provedResult] MultiItemResponse provedResult
                         * @property {org.dash.platform.dapi.v0.IResponseMetadata|null} [metadata] MultiItemResponse metadata
                         */

                        /**
                         * Constructs a new MultiItemResponse.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a MultiItemResponse.
                         * @implements IMultiItemResponse
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.IMultiItemResponse=} [properties] Properties to set
                         */
                        function MultiItemResponse(properties) {
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * MultiItemResponse nonProvedResults.
                         * @member {org.dash.platform.dapi.v0.IResultList|null|undefined} nonProvedResults
                         * @memberof org.dash.platform.dapi.v0.MultiItemResponse
                         * @instance
                         */
                        MultiItemResponse.prototype.nonProvedResults = null;

                        /**
                         * MultiItemResponse provedResult.
                         * @member {org.dash.platform.dapi.v0.IProvedResult|null|undefined} provedResult
                         * @memberof org.dash.platform.dapi.v0.MultiItemResponse
                         * @instance
                         */
                        MultiItemResponse.prototype.provedResult = null;

                        /**
                         * MultiItemResponse metadata.
                         * @member {org.dash.platform.dapi.v0.IResponseMetadata|null|undefined} metadata
                         * @memberof org.dash.platform.dapi.v0.MultiItemResponse
                         * @instance
                         */
                        MultiItemResponse.prototype.metadata = null;

                        // OneOf field names bound to virtual getters and setters
                        var $oneOfFields;

                        /**
                         * MultiItemResponse result.
                         * @member {"nonProvedResults"|"provedResult"|undefined} result
                         * @memberof org.dash.platform.dapi.v0.MultiItemResponse
                         * @instance
                         */
                        Object.defineProperty(MultiItemResponse.prototype, "result", {
                            get: $util.oneOfGetter($oneOfFields = ["nonProvedResults", "provedResult"]),
                            set: $util.oneOfSetter($oneOfFields)
                        });

                        /**
                         * Creates a new MultiItemResponse instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.MultiItemResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.IMultiItemResponse=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.MultiItemResponse} MultiItemResponse instance
                         */
                        MultiItemResponse.create = function create(properties) {
                            return new MultiItemResponse(properties);
                        };

                        /**
                         * Encodes the specified MultiItemResponse message. Does not implicitly {@link org.dash.platform.dapi.v0.MultiItemResponse.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.MultiItemResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.IMultiItemResponse} message MultiItemResponse message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        MultiItemResponse.encode = function encode(message, writer) {
                            if (!writer)
                                writer = $Writer.create();
                            if (message.nonProvedResults != null && Object.hasOwnProperty.call(message, "nonProvedResults"))
                                $root.org.dash.platform.dapi.v0.ResultList.encode(message.nonProvedResults, writer.uint32(/* id 1, wireType 2 =*/10).fork()).ldelim();
                            if (message.provedResult != null && Object.hasOwnProperty.call(message, "provedResult"))
                                $root.org.dash.platform.dapi.v0.ProvedResult.encode(message.provedResult, writer.uint32(/* id 2, wireType 2 =*/18).fork()).ldelim();
                            if (message.metadata != null && Object.hasOwnProperty.call(message, "metadata"))
                                $root.org.dash.platform.dapi.v0.ResponseMetadata.encode(message.metadata, writer.uint32(/* id 3, wireType 2 =*/26).fork()).ldelim();
                            return writer;
                        };

                        /**
                         * Encodes the specified MultiItemResponse message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.MultiItemResponse.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.MultiItemResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.IMultiItemResponse} message MultiItemResponse message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        MultiItemResponse.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes a MultiItemResponse message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.MultiItemResponse
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.MultiItemResponse} MultiItemResponse
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        MultiItemResponse.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.MultiItemResponse();
                            while (reader.pos < end) {
                                var tag = reader.uint32();
                                switch (tag >>> 3) {
                                case 1:
                                    message.nonProvedResults = $root.org.dash.platform.dapi.v0.ResultList.decode(reader, reader.uint32());
                                    break;
                                case 2:
                                    message.provedResult = $root.org.dash.platform.dapi.v0.ProvedResult.decode(reader, reader.uint32());
                                    break;
                                case 3:
                                    message.metadata = $root.org.dash.platform.dapi.v0.ResponseMetadata.decode(reader, reader.uint32());
                                    break;
                                default:
                                    reader.skipType(tag & 7);
                                    break;
                                }
                            }
                            return message;
                        };

                        /**
                         * Decodes a MultiItemResponse message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.MultiItemResponse
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.MultiItemResponse} MultiItemResponse
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        MultiItemResponse.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies a MultiItemResponse message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.MultiItemResponse
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        MultiItemResponse.verify = function verify(message) {
                            if (typeof message !== "object" || message === null)
                                return "object expected";
                            var properties = {};
                            if (message.nonProvedResults != null && message.hasOwnProperty("nonProvedResults")) {
                                properties.result = 1;
                                {
                                    var error = $root.org.dash.platform.dapi.v0.ResultList.verify(message.nonProvedResults);
                                    if (error)
                                        return "nonProvedResults." + error;
                                }
                            }
                            if (message.provedResult != null && message.hasOwnProperty("provedResult")) {
                                if (properties.result === 1)
                                    return "result: multiple values";
                                properties.result = 1;
                                {
                                    var error = $root.org.dash.platform.dapi.v0.ProvedResult.verify(message.provedResult);
                                    if (error)
                                        return "provedResult." + error;
                                }
                            }
                            if (message.metadata != null && message.hasOwnProperty("metadata")) {
                                var error = $root.org.dash.platform.dapi.v0.ResponseMetadata.verify(message.metadata);
                                if (error)
                                    return "metadata." + error;
                            }
                            return null;
                        };

                        /**
                         * Creates a MultiItemResponse message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.MultiItemResponse
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.MultiItemResponse} MultiItemResponse
                         */
                        MultiItemResponse.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.MultiItemResponse)
                                return object;
                            var message = new $root.org.dash.platform.dapi.v0.MultiItemResponse();
                            if (object.nonProvedResults != null) {
                                if (typeof object.nonProvedResults !== "object")
                                    throw TypeError(".org.dash.platform.dapi.v0.MultiItemResponse.nonProvedResults: object expected");
                                message.nonProvedResults = $root.org.dash.platform.dapi.v0.ResultList.fromObject(object.nonProvedResults);
                            }
                            if (object.provedResult != null) {
                                if (typeof object.provedResult !== "object")
                                    throw TypeError(".org.dash.platform.dapi.v0.MultiItemResponse.provedResult: object expected");
                                message.provedResult = $root.org.dash.platform.dapi.v0.ProvedResult.fromObject(object.provedResult);
                            }
                            if (object.metadata != null) {
                                if (typeof object.metadata !== "object")
                                    throw TypeError(".org.dash.platform.dapi.v0.MultiItemResponse.metadata: object expected");
                                message.metadata = $root.org.dash.platform.dapi.v0.ResponseMetadata.fromObject(object.metadata);
                            }
                            return message;
                        };

                        /**
                         * Creates a plain object from a MultiItemResponse message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.MultiItemResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.MultiItemResponse} message MultiItemResponse
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        MultiItemResponse.toObject = function toObject(message, options) {
                            if (!options)
                                options = {};
                            var object = {};
                            if (options.defaults)
                                object.metadata = null;
                            if (message.nonProvedResults != null && message.hasOwnProperty("nonProvedResults")) {
                                object.nonProvedResults = $root.org.dash.platform.dapi.v0.ResultList.toObject(message.nonProvedResults, options);
                                if (options.oneofs)
                                    object.result = "nonProvedResults";
                            }
                            if (message.provedResult != null && message.hasOwnProperty("provedResult")) {
                                object.provedResult = $root.org.dash.platform.dapi.v0.ProvedResult.toObject(message.provedResult, options);
                                if (options.oneofs)
                                    object.result = "provedResult";
                            }
                            if (message.metadata != null && message.hasOwnProperty("metadata"))
                                object.metadata = $root.org.dash.platform.dapi.v0.ResponseMetadata.toObject(message.metadata, options);
                            return object;
                        };

                        /**
                         * Converts this MultiItemResponse to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.MultiItemResponse
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        MultiItemResponse.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        return MultiItemResponse;
                    })();

                    v0.GetSingleItemRequest = (function() {

                        /**
                         * Properties of a GetSingleItemRequest.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface IGetSingleItemRequest
                         * @property {Uint8Array|null} [id] GetSingleItemRequest id
                         * @property {boolean|null} [prove] GetSingleItemRequest prove
                         */

                        /**
                         * Constructs a new GetSingleItemRequest.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a GetSingleItemRequest.
                         * @implements IGetSingleItemRequest
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.IGetSingleItemRequest=} [properties] Properties to set
                         */
                        function GetSingleItemRequest(properties) {
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * GetSingleItemRequest id.
                         * @member {Uint8Array} id
                         * @memberof org.dash.platform.dapi.v0.GetSingleItemRequest
                         * @instance
                         */
                        GetSingleItemRequest.prototype.id = $util.newBuffer([]);

                        /**
                         * GetSingleItemRequest prove.
                         * @member {boolean} prove
                         * @memberof org.dash.platform.dapi.v0.GetSingleItemRequest
                         * @instance
                         */
                        GetSingleItemRequest.prototype.prove = false;

                        /**
                         * Creates a new GetSingleItemRequest instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.GetSingleItemRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetSingleItemRequest=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.GetSingleItemRequest} GetSingleItemRequest instance
                         */
                        GetSingleItemRequest.create = function create(properties) {
                            return new GetSingleItemRequest(properties);
                        };

                        /**
                         * Encodes the specified GetSingleItemRequest message. Does not implicitly {@link org.dash.platform.dapi.v0.GetSingleItemRequest.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.GetSingleItemRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetSingleItemRequest} message GetSingleItemRequest message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        GetSingleItemRequest.encode = function encode(message, writer) {
                            if (!writer)
                                writer = $Writer.create();
                            if (message.id != null && Object.hasOwnProperty.call(message, "id"))
                                writer.uint32(/* id 1, wireType 2 =*/10).bytes(message.id);
                            if (message.prove != null && Object.hasOwnProperty.call(message, "prove"))
                                writer.uint32(/* id 2, wireType 0 =*/16).bool(message.prove);
                            return writer;
                        };

                        /**
                         * Encodes the specified GetSingleItemRequest message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.GetSingleItemRequest.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.GetSingleItemRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetSingleItemRequest} message GetSingleItemRequest message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        GetSingleItemRequest.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes a GetSingleItemRequest message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.GetSingleItemRequest
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.GetSingleItemRequest} GetSingleItemRequest
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        GetSingleItemRequest.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.GetSingleItemRequest();
                            while (reader.pos < end) {
                                var tag = reader.uint32();
                                switch (tag >>> 3) {
                                case 1:
                                    message.id = reader.bytes();
                                    break;
                                case 2:
                                    message.prove = reader.bool();
                                    break;
                                default:
                                    reader.skipType(tag & 7);
                                    break;
                                }
                            }
                            return message;
                        };

                        /**
                         * Decodes a GetSingleItemRequest message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.GetSingleItemRequest
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.GetSingleItemRequest} GetSingleItemRequest
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        GetSingleItemRequest.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies a GetSingleItemRequest message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.GetSingleItemRequest
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        GetSingleItemRequest.verify = function verify(message) {
                            if (typeof message !== "object" || message === null)
                                return "object expected";
                            if (message.id != null && message.hasOwnProperty("id"))
                                if (!(message.id && typeof message.id.length === "number" || $util.isString(message.id)))
                                    return "id: buffer expected";
                            if (message.prove != null && message.hasOwnProperty("prove"))
                                if (typeof message.prove !== "boolean")
                                    return "prove: boolean expected";
                            return null;
                        };

                        /**
                         * Creates a GetSingleItemRequest message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.GetSingleItemRequest
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.GetSingleItemRequest} GetSingleItemRequest
                         */
                        GetSingleItemRequest.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.GetSingleItemRequest)
                                return object;
                            var message = new $root.org.dash.platform.dapi.v0.GetSingleItemRequest();
                            if (object.id != null)
                                if (typeof object.id === "string")
                                    $util.base64.decode(object.id, message.id = $util.newBuffer($util.base64.length(object.id)), 0);
                                else if (object.id.length >= 0)
                                    message.id = object.id;
                            if (object.prove != null)
                                message.prove = Boolean(object.prove);
                            return message;
                        };

                        /**
                         * Creates a plain object from a GetSingleItemRequest message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.GetSingleItemRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.GetSingleItemRequest} message GetSingleItemRequest
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        GetSingleItemRequest.toObject = function toObject(message, options) {
                            if (!options)
                                options = {};
                            var object = {};
                            if (options.defaults) {
                                if (options.bytes === String)
                                    object.id = "";
                                else {
                                    object.id = [];
                                    if (options.bytes !== Array)
                                        object.id = $util.newBuffer(object.id);
                                }
                                object.prove = false;
                            }
                            if (message.id != null && message.hasOwnProperty("id"))
                                object.id = options.bytes === String ? $util.base64.encode(message.id, 0, message.id.length) : options.bytes === Array ? Array.prototype.slice.call(message.id) : message.id;
                            if (message.prove != null && message.hasOwnProperty("prove"))
                                object.prove = message.prove;
                            return object;
                        };

                        /**
                         * Converts this GetSingleItemRequest to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.GetSingleItemRequest
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        GetSingleItemRequest.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        return GetSingleItemRequest;
                    })();

                    v0.GetMultiItemRequest = (function() {

                        /**
                         * Properties of a GetMultiItemRequest.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface IGetMultiItemRequest
                         * @property {Array.<Uint8Array>|null} [ids] GetMultiItemRequest ids
                         * @property {boolean|null} [prove] GetMultiItemRequest prove
                         */

                        /**
                         * Constructs a new GetMultiItemRequest.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a GetMultiItemRequest.
                         * @implements IGetMultiItemRequest
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.IGetMultiItemRequest=} [properties] Properties to set
                         */
                        function GetMultiItemRequest(properties) {
                            this.ids = [];
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * GetMultiItemRequest ids.
                         * @member {Array.<Uint8Array>} ids
                         * @memberof org.dash.platform.dapi.v0.GetMultiItemRequest
                         * @instance
                         */
                        GetMultiItemRequest.prototype.ids = $util.emptyArray;

                        /**
                         * GetMultiItemRequest prove.
                         * @member {boolean} prove
                         * @memberof org.dash.platform.dapi.v0.GetMultiItemRequest
                         * @instance
                         */
                        GetMultiItemRequest.prototype.prove = false;

                        /**
                         * Creates a new GetMultiItemRequest instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.GetMultiItemRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetMultiItemRequest=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.GetMultiItemRequest} GetMultiItemRequest instance
                         */
                        GetMultiItemRequest.create = function create(properties) {
                            return new GetMultiItemRequest(properties);
                        };

                        /**
                         * Encodes the specified GetMultiItemRequest message. Does not implicitly {@link org.dash.platform.dapi.v0.GetMultiItemRequest.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.GetMultiItemRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetMultiItemRequest} message GetMultiItemRequest message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        GetMultiItemRequest.encode = function encode(message, writer) {
                            if (!writer)
                                writer = $Writer.create();
                            if (message.ids != null && message.ids.length)
                                for (var i = 0; i < message.ids.length; ++i)
                                    writer.uint32(/* id 1, wireType 2 =*/10).bytes(message.ids[i]);
                            if (message.prove != null && Object.hasOwnProperty.call(message, "prove"))
                                writer.uint32(/* id 2, wireType 0 =*/16).bool(message.prove);
                            return writer;
                        };

                        /**
                         * Encodes the specified GetMultiItemRequest message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.GetMultiItemRequest.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.GetMultiItemRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetMultiItemRequest} message GetMultiItemRequest message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        GetMultiItemRequest.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes a GetMultiItemRequest message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.GetMultiItemRequest
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.GetMultiItemRequest} GetMultiItemRequest
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        GetMultiItemRequest.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.GetMultiItemRequest();
                            while (reader.pos < end) {
                                var tag = reader.uint32();
                                switch (tag >>> 3) {
                                case 1:
                                    if (!(message.ids && message.ids.length))
                                        message.ids = [];
                                    message.ids.push(reader.bytes());
                                    break;
                                case 2:
                                    message.prove = reader.bool();
                                    break;
                                default:
                                    reader.skipType(tag & 7);
                                    break;
                                }
                            }
                            return message;
                        };

                        /**
                         * Decodes a GetMultiItemRequest message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.GetMultiItemRequest
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.GetMultiItemRequest} GetMultiItemRequest
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        GetMultiItemRequest.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies a GetMultiItemRequest message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.GetMultiItemRequest
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        GetMultiItemRequest.verify = function verify(message) {
                            if (typeof message !== "object" || message === null)
                                return "object expected";
                            if (message.ids != null && message.hasOwnProperty("ids")) {
                                if (!Array.isArray(message.ids))
                                    return "ids: array expected";
                                for (var i = 0; i < message.ids.length; ++i)
                                    if (!(message.ids[i] && typeof message.ids[i].length === "number" || $util.isString(message.ids[i])))
                                        return "ids: buffer[] expected";
                            }
                            if (message.prove != null && message.hasOwnProperty("prove"))
                                if (typeof message.prove !== "boolean")
                                    return "prove: boolean expected";
                            return null;
                        };

                        /**
                         * Creates a GetMultiItemRequest message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.GetMultiItemRequest
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.GetMultiItemRequest} GetMultiItemRequest
                         */
                        GetMultiItemRequest.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.GetMultiItemRequest)
                                return object;
                            var message = new $root.org.dash.platform.dapi.v0.GetMultiItemRequest();
                            if (object.ids) {
                                if (!Array.isArray(object.ids))
                                    throw TypeError(".org.dash.platform.dapi.v0.GetMultiItemRequest.ids: array expected");
                                message.ids = [];
                                for (var i = 0; i < object.ids.length; ++i)
                                    if (typeof object.ids[i] === "string")
                                        $util.base64.decode(object.ids[i], message.ids[i] = $util.newBuffer($util.base64.length(object.ids[i])), 0);
                                    else if (object.ids[i].length >= 0)
                                        message.ids[i] = object.ids[i];
                            }
                            if (object.prove != null)
                                message.prove = Boolean(object.prove);
                            return message;
                        };

                        /**
                         * Creates a plain object from a GetMultiItemRequest message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.GetMultiItemRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.GetMultiItemRequest} message GetMultiItemRequest
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        GetMultiItemRequest.toObject = function toObject(message, options) {
                            if (!options)
                                options = {};
                            var object = {};
                            if (options.arrays || options.defaults)
                                object.ids = [];
                            if (options.defaults)
                                object.prove = false;
                            if (message.ids && message.ids.length) {
                                object.ids = [];
                                for (var j = 0; j < message.ids.length; ++j)
                                    object.ids[j] = options.bytes === String ? $util.base64.encode(message.ids[j], 0, message.ids[j].length) : options.bytes === Array ? Array.prototype.slice.call(message.ids[j]) : message.ids[j];
                            }
                            if (message.prove != null && message.hasOwnProperty("prove"))
                                object.prove = message.prove;
                            return object;
                        };

                        /**
                         * Converts this GetMultiItemRequest to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.GetMultiItemRequest
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        GetMultiItemRequest.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        return GetMultiItemRequest;
                    })();

                    v0.GetDocumentsRequest = (function() {

                        /**
                         * Properties of a GetDocumentsRequest.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface IGetDocumentsRequest
                         * @property {Uint8Array|null} [dataContractId] GetDocumentsRequest dataContractId
                         * @property {string|null} [documentType] GetDocumentsRequest documentType
                         * @property {Uint8Array|null} [where] GetDocumentsRequest where
                         * @property {Uint8Array|null} [orderBy] GetDocumentsRequest orderBy
                         * @property {number|null} [limit] GetDocumentsRequest limit
                         * @property {Uint8Array|null} [startAfter] GetDocumentsRequest startAfter
                         * @property {Uint8Array|null} [startAt] GetDocumentsRequest startAt
                         * @property {boolean|null} [prove] GetDocumentsRequest prove
                         */

                        /**
                         * Constructs a new GetDocumentsRequest.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a GetDocumentsRequest.
                         * @implements IGetDocumentsRequest
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.IGetDocumentsRequest=} [properties] Properties to set
                         */
                        function GetDocumentsRequest(properties) {
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * GetDocumentsRequest dataContractId.
                         * @member {Uint8Array} dataContractId
                         * @memberof org.dash.platform.dapi.v0.GetDocumentsRequest
                         * @instance
                         */
                        GetDocumentsRequest.prototype.dataContractId = $util.newBuffer([]);

                        /**
                         * GetDocumentsRequest documentType.
                         * @member {string} documentType
                         * @memberof org.dash.platform.dapi.v0.GetDocumentsRequest
                         * @instance
                         */
                        GetDocumentsRequest.prototype.documentType = "";

                        /**
                         * GetDocumentsRequest where.
                         * @member {Uint8Array} where
                         * @memberof org.dash.platform.dapi.v0.GetDocumentsRequest
                         * @instance
                         */
                        GetDocumentsRequest.prototype.where = $util.newBuffer([]);

                        /**
                         * GetDocumentsRequest orderBy.
                         * @member {Uint8Array} orderBy
                         * @memberof org.dash.platform.dapi.v0.GetDocumentsRequest
                         * @instance
                         */
                        GetDocumentsRequest.prototype.orderBy = $util.newBuffer([]);

                        /**
                         * GetDocumentsRequest limit.
                         * @member {number} limit
                         * @memberof org.dash.platform.dapi.v0.GetDocumentsRequest
                         * @instance
                         */
                        GetDocumentsRequest.prototype.limit = 0;

                        /**
                         * GetDocumentsRequest startAfter.
                         * @member {Uint8Array} startAfter
                         * @memberof org.dash.platform.dapi.v0.GetDocumentsRequest
                         * @instance
                         */
                        GetDocumentsRequest.prototype.startAfter = $util.newBuffer([]);

                        /**
                         * GetDocumentsRequest startAt.
                         * @member {Uint8Array} startAt
                         * @memberof org.dash.platform.dapi.v0.GetDocumentsRequest
                         * @instance
                         */
                        GetDocumentsRequest.prototype.startAt = $util.newBuffer([]);

                        /**
                         * GetDocumentsRequest prove.
                         * @member {boolean} prove
                         * @memberof org.dash.platform.dapi.v0.GetDocumentsRequest
                         * @instance
                         */
                        GetDocumentsRequest.prototype.prove = false;

                        // OneOf field names bound to virtual getters and setters
                        var $oneOfFields;

                        /**
                         * GetDocumentsRequest start.
                         * @member {"startAfter"|"startAt"|undefined} start
                         * @memberof org.dash.platform.dapi.v0.GetDocumentsRequest
                         * @instance
                         */
                        Object.defineProperty(GetDocumentsRequest.prototype, "start", {
                            get: $util.oneOfGetter($oneOfFields = ["startAfter", "startAt"]),
                            set: $util.oneOfSetter($oneOfFields)
                        });

                        /**
                         * Creates a new GetDocumentsRequest instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.GetDocumentsRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetDocumentsRequest=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.GetDocumentsRequest} GetDocumentsRequest instance
                         */
                        GetDocumentsRequest.create = function create(properties) {
                            return new GetDocumentsRequest(properties);
                        };

                        /**
                         * Encodes the specified GetDocumentsRequest message. Does not implicitly {@link org.dash.platform.dapi.v0.GetDocumentsRequest.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.GetDocumentsRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetDocumentsRequest} message GetDocumentsRequest message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        GetDocumentsRequest.encode = function encode(message, writer) {
                            if (!writer)
                                writer = $Writer.create();
                            if (message.dataContractId != null && Object.hasOwnProperty.call(message, "dataContractId"))
                                writer.uint32(/* id 1, wireType 2 =*/10).bytes(message.dataContractId);
                            if (message.documentType != null && Object.hasOwnProperty.call(message, "documentType"))
                                writer.uint32(/* id 2, wireType 2 =*/18).string(message.documentType);
                            if (message.where != null && Object.hasOwnProperty.call(message, "where"))
                                writer.uint32(/* id 3, wireType 2 =*/26).bytes(message.where);
                            if (message.orderBy != null && Object.hasOwnProperty.call(message, "orderBy"))
                                writer.uint32(/* id 4, wireType 2 =*/34).bytes(message.orderBy);
                            if (message.limit != null && Object.hasOwnProperty.call(message, "limit"))
                                writer.uint32(/* id 5, wireType 0 =*/40).uint32(message.limit);
                            if (message.startAfter != null && Object.hasOwnProperty.call(message, "startAfter"))
                                writer.uint32(/* id 6, wireType 2 =*/50).bytes(message.startAfter);
                            if (message.startAt != null && Object.hasOwnProperty.call(message, "startAt"))
                                writer.uint32(/* id 7, wireType 2 =*/58).bytes(message.startAt);
                            if (message.prove != null && Object.hasOwnProperty.call(message, "prove"))
                                writer.uint32(/* id 8, wireType 0 =*/64).bool(message.prove);
                            return writer;
                        };

                        /**
                         * Encodes the specified GetDocumentsRequest message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.GetDocumentsRequest.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.GetDocumentsRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetDocumentsRequest} message GetDocumentsRequest message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        GetDocumentsRequest.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes a GetDocumentsRequest message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.GetDocumentsRequest
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.GetDocumentsRequest} GetDocumentsRequest
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        GetDocumentsRequest.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.GetDocumentsRequest();
                            while (reader.pos < end) {
                                var tag = reader.uint32();
                                switch (tag >>> 3) {
                                case 1:
                                    message.dataContractId = reader.bytes();
                                    break;
                                case 2:
                                    message.documentType = reader.string();
                                    break;
                                case 3:
                                    message.where = reader.bytes();
                                    break;
                                case 4:
                                    message.orderBy = reader.bytes();
                                    break;
                                case 5:
                                    message.limit = reader.uint32();
                                    break;
                                case 6:
                                    message.startAfter = reader.bytes();
                                    break;
                                case 7:
                                    message.startAt = reader.bytes();
                                    break;
                                case 8:
                                    message.prove = reader.bool();
                                    break;
                                default:
                                    reader.skipType(tag & 7);
                                    break;
                                }
                            }
                            return message;
                        };

                        /**
                         * Decodes a GetDocumentsRequest message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.GetDocumentsRequest
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.GetDocumentsRequest} GetDocumentsRequest
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        GetDocumentsRequest.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies a GetDocumentsRequest message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.GetDocumentsRequest
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        GetDocumentsRequest.verify = function verify(message) {
                            if (typeof message !== "object" || message === null)
                                return "object expected";
                            var properties = {};
                            if (message.dataContractId != null && message.hasOwnProperty("dataContractId"))
                                if (!(message.dataContractId && typeof message.dataContractId.length === "number" || $util.isString(message.dataContractId)))
                                    return "dataContractId: buffer expected";
                            if (message.documentType != null && message.hasOwnProperty("documentType"))
                                if (!$util.isString(message.documentType))
                                    return "documentType: string expected";
                            if (message.where != null && message.hasOwnProperty("where"))
                                if (!(message.where && typeof message.where.length === "number" || $util.isString(message.where)))
                                    return "where: buffer expected";
                            if (message.orderBy != null && message.hasOwnProperty("orderBy"))
                                if (!(message.orderBy && typeof message.orderBy.length === "number" || $util.isString(message.orderBy)))
                                    return "orderBy: buffer expected";
                            if (message.limit != null && message.hasOwnProperty("limit"))
                                if (!$util.isInteger(message.limit))
                                    return "limit: integer expected";
                            if (message.startAfter != null && message.hasOwnProperty("startAfter")) {
                                properties.start = 1;
                                if (!(message.startAfter && typeof message.startAfter.length === "number" || $util.isString(message.startAfter)))
                                    return "startAfter: buffer expected";
                            }
                            if (message.startAt != null && message.hasOwnProperty("startAt")) {
                                if (properties.start === 1)
                                    return "start: multiple values";
                                properties.start = 1;
                                if (!(message.startAt && typeof message.startAt.length === "number" || $util.isString(message.startAt)))
                                    return "startAt: buffer expected";
                            }
                            if (message.prove != null && message.hasOwnProperty("prove"))
                                if (typeof message.prove !== "boolean")
                                    return "prove: boolean expected";
                            return null;
                        };

                        /**
                         * Creates a GetDocumentsRequest message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.GetDocumentsRequest
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.GetDocumentsRequest} GetDocumentsRequest
                         */
                        GetDocumentsRequest.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.GetDocumentsRequest)
                                return object;
                            var message = new $root.org.dash.platform.dapi.v0.GetDocumentsRequest();
                            if (object.dataContractId != null)
                                if (typeof object.dataContractId === "string")
                                    $util.base64.decode(object.dataContractId, message.dataContractId = $util.newBuffer($util.base64.length(object.dataContractId)), 0);
                                else if (object.dataContractId.length >= 0)
                                    message.dataContractId = object.dataContractId;
                            if (object.documentType != null)
                                message.documentType = String(object.documentType);
                            if (object.where != null)
                                if (typeof object.where === "string")
                                    $util.base64.decode(object.where, message.where = $util.newBuffer($util.base64.length(object.where)), 0);
                                else if (object.where.length >= 0)
                                    message.where = object.where;
                            if (object.orderBy != null)
                                if (typeof object.orderBy === "string")
                                    $util.base64.decode(object.orderBy, message.orderBy = $util.newBuffer($util.base64.length(object.orderBy)), 0);
                                else if (object.orderBy.length >= 0)
                                    message.orderBy = object.orderBy;
                            if (object.limit != null)
                                message.limit = object.limit >>> 0;
                            if (object.startAfter != null)
                                if (typeof object.startAfter === "string")
                                    $util.base64.decode(object.startAfter, message.startAfter = $util.newBuffer($util.base64.length(object.startAfter)), 0);
                                else if (object.startAfter.length >= 0)
                                    message.startAfter = object.startAfter;
                            if (object.startAt != null)
                                if (typeof object.startAt === "string")
                                    $util.base64.decode(object.startAt, message.startAt = $util.newBuffer($util.base64.length(object.startAt)), 0);
                                else if (object.startAt.length >= 0)
                                    message.startAt = object.startAt;
                            if (object.prove != null)
                                message.prove = Boolean(object.prove);
                            return message;
                        };

                        /**
                         * Creates a plain object from a GetDocumentsRequest message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.GetDocumentsRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.GetDocumentsRequest} message GetDocumentsRequest
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        GetDocumentsRequest.toObject = function toObject(message, options) {
                            if (!options)
                                options = {};
                            var object = {};
                            if (options.defaults) {
                                if (options.bytes === String)
                                    object.dataContractId = "";
                                else {
                                    object.dataContractId = [];
                                    if (options.bytes !== Array)
                                        object.dataContractId = $util.newBuffer(object.dataContractId);
                                }
                                object.documentType = "";
                                if (options.bytes === String)
                                    object.where = "";
                                else {
                                    object.where = [];
                                    if (options.bytes !== Array)
                                        object.where = $util.newBuffer(object.where);
                                }
                                if (options.bytes === String)
                                    object.orderBy = "";
                                else {
                                    object.orderBy = [];
                                    if (options.bytes !== Array)
                                        object.orderBy = $util.newBuffer(object.orderBy);
                                }
                                object.limit = 0;
                                object.prove = false;
                            }
                            if (message.dataContractId != null && message.hasOwnProperty("dataContractId"))
                                object.dataContractId = options.bytes === String ? $util.base64.encode(message.dataContractId, 0, message.dataContractId.length) : options.bytes === Array ? Array.prototype.slice.call(message.dataContractId) : message.dataContractId;
                            if (message.documentType != null && message.hasOwnProperty("documentType"))
                                object.documentType = message.documentType;
                            if (message.where != null && message.hasOwnProperty("where"))
                                object.where = options.bytes === String ? $util.base64.encode(message.where, 0, message.where.length) : options.bytes === Array ? Array.prototype.slice.call(message.where) : message.where;
                            if (message.orderBy != null && message.hasOwnProperty("orderBy"))
                                object.orderBy = options.bytes === String ? $util.base64.encode(message.orderBy, 0, message.orderBy.length) : options.bytes === Array ? Array.prototype.slice.call(message.orderBy) : message.orderBy;
                            if (message.limit != null && message.hasOwnProperty("limit"))
                                object.limit = message.limit;
                            if (message.startAfter != null && message.hasOwnProperty("startAfter")) {
                                object.startAfter = options.bytes === String ? $util.base64.encode(message.startAfter, 0, message.startAfter.length) : options.bytes === Array ? Array.prototype.slice.call(message.startAfter) : message.startAfter;
                                if (options.oneofs)
                                    object.start = "startAfter";
                            }
                            if (message.startAt != null && message.hasOwnProperty("startAt")) {
                                object.startAt = options.bytes === String ? $util.base64.encode(message.startAt, 0, message.startAt.length) : options.bytes === Array ? Array.prototype.slice.call(message.startAt) : message.startAt;
                                if (options.oneofs)
                                    object.start = "startAt";
                            }
                            if (message.prove != null && message.hasOwnProperty("prove"))
                                object.prove = message.prove;
                            return object;
                        };

                        /**
                         * Converts this GetDocumentsRequest to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.GetDocumentsRequest
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        GetDocumentsRequest.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        return GetDocumentsRequest;
                    })();

                    v0.WaitForStateTransitionResultRequest = (function() {

                        /**
                         * Properties of a WaitForStateTransitionResultRequest.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface IWaitForStateTransitionResultRequest
                         * @property {Uint8Array|null} [stateTransitionHash] WaitForStateTransitionResultRequest stateTransitionHash
                         * @property {boolean|null} [prove] WaitForStateTransitionResultRequest prove
                         */

                        /**
                         * Constructs a new WaitForStateTransitionResultRequest.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a WaitForStateTransitionResultRequest.
                         * @implements IWaitForStateTransitionResultRequest
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.IWaitForStateTransitionResultRequest=} [properties] Properties to set
                         */
                        function WaitForStateTransitionResultRequest(properties) {
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * WaitForStateTransitionResultRequest stateTransitionHash.
                         * @member {Uint8Array} stateTransitionHash
                         * @memberof org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest
                         * @instance
                         */
                        WaitForStateTransitionResultRequest.prototype.stateTransitionHash = $util.newBuffer([]);

                        /**
                         * WaitForStateTransitionResultRequest prove.
                         * @member {boolean} prove
                         * @memberof org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest
                         * @instance
                         */
                        WaitForStateTransitionResultRequest.prototype.prove = false;

                        /**
                         * Creates a new WaitForStateTransitionResultRequest instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.IWaitForStateTransitionResultRequest=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest} WaitForStateTransitionResultRequest instance
                         */
                        WaitForStateTransitionResultRequest.create = function create(properties) {
                            return new WaitForStateTransitionResultRequest(properties);
                        };

                        /**
                         * Encodes the specified WaitForStateTransitionResultRequest message. Does not implicitly {@link org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.IWaitForStateTransitionResultRequest} message WaitForStateTransitionResultRequest message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        WaitForStateTransitionResultRequest.encode = function encode(message, writer) {
                            if (!writer)
                                writer = $Writer.create();
                            if (message.stateTransitionHash != null && Object.hasOwnProperty.call(message, "stateTransitionHash"))
                                writer.uint32(/* id 1, wireType 2 =*/10).bytes(message.stateTransitionHash);
                            if (message.prove != null && Object.hasOwnProperty.call(message, "prove"))
                                writer.uint32(/* id 2, wireType 0 =*/16).bool(message.prove);
                            return writer;
                        };

                        /**
                         * Encodes the specified WaitForStateTransitionResultRequest message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.IWaitForStateTransitionResultRequest} message WaitForStateTransitionResultRequest message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        WaitForStateTransitionResultRequest.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes a WaitForStateTransitionResultRequest message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest} WaitForStateTransitionResultRequest
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        WaitForStateTransitionResultRequest.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest();
                            while (reader.pos < end) {
                                var tag = reader.uint32();
                                switch (tag >>> 3) {
                                case 1:
                                    message.stateTransitionHash = reader.bytes();
                                    break;
                                case 2:
                                    message.prove = reader.bool();
                                    break;
                                default:
                                    reader.skipType(tag & 7);
                                    break;
                                }
                            }
                            return message;
                        };

                        /**
                         * Decodes a WaitForStateTransitionResultRequest message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest} WaitForStateTransitionResultRequest
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        WaitForStateTransitionResultRequest.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies a WaitForStateTransitionResultRequest message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        WaitForStateTransitionResultRequest.verify = function verify(message) {
                            if (typeof message !== "object" || message === null)
                                return "object expected";
                            if (message.stateTransitionHash != null && message.hasOwnProperty("stateTransitionHash"))
                                if (!(message.stateTransitionHash && typeof message.stateTransitionHash.length === "number" || $util.isString(message.stateTransitionHash)))
                                    return "stateTransitionHash: buffer expected";
                            if (message.prove != null && message.hasOwnProperty("prove"))
                                if (typeof message.prove !== "boolean")
                                    return "prove: boolean expected";
                            return null;
                        };

                        /**
                         * Creates a WaitForStateTransitionResultRequest message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest} WaitForStateTransitionResultRequest
                         */
                        WaitForStateTransitionResultRequest.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest)
                                return object;
                            var message = new $root.org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest();
                            if (object.stateTransitionHash != null)
                                if (typeof object.stateTransitionHash === "string")
                                    $util.base64.decode(object.stateTransitionHash, message.stateTransitionHash = $util.newBuffer($util.base64.length(object.stateTransitionHash)), 0);
                                else if (object.stateTransitionHash.length >= 0)
                                    message.stateTransitionHash = object.stateTransitionHash;
                            if (object.prove != null)
                                message.prove = Boolean(object.prove);
                            return message;
                        };

                        /**
                         * Creates a plain object from a WaitForStateTransitionResultRequest message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest} message WaitForStateTransitionResultRequest
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        WaitForStateTransitionResultRequest.toObject = function toObject(message, options) {
                            if (!options)
                                options = {};
                            var object = {};
                            if (options.defaults) {
                                if (options.bytes === String)
                                    object.stateTransitionHash = "";
                                else {
                                    object.stateTransitionHash = [];
                                    if (options.bytes !== Array)
                                        object.stateTransitionHash = $util.newBuffer(object.stateTransitionHash);
                                }
                                object.prove = false;
                            }
                            if (message.stateTransitionHash != null && message.hasOwnProperty("stateTransitionHash"))
                                object.stateTransitionHash = options.bytes === String ? $util.base64.encode(message.stateTransitionHash, 0, message.stateTransitionHash.length) : options.bytes === Array ? Array.prototype.slice.call(message.stateTransitionHash) : message.stateTransitionHash;
                            if (message.prove != null && message.hasOwnProperty("prove"))
                                object.prove = message.prove;
                            return object;
                        };

                        /**
                         * Converts this WaitForStateTransitionResultRequest to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.WaitForStateTransitionResultRequest
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        WaitForStateTransitionResultRequest.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        return WaitForStateTransitionResultRequest;
                    })();

                    v0.WaitForStateTransitionResultResponse = (function() {

                        /**
                         * Properties of a WaitForStateTransitionResultResponse.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface IWaitForStateTransitionResultResponse
                         * @property {org.dash.platform.dapi.v0.IStateTransitionBroadcastError|null} [error] WaitForStateTransitionResultResponse error
                         * @property {org.dash.platform.dapi.v0.IProvedResult|null} [proof] WaitForStateTransitionResultResponse proof
                         * @property {org.dash.platform.dapi.v0.IResponseMetadata|null} [metadata] WaitForStateTransitionResultResponse metadata
                         */

                        /**
                         * Constructs a new WaitForStateTransitionResultResponse.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a WaitForStateTransitionResultResponse.
                         * @implements IWaitForStateTransitionResultResponse
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.IWaitForStateTransitionResultResponse=} [properties] Properties to set
                         */
                        function WaitForStateTransitionResultResponse(properties) {
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * WaitForStateTransitionResultResponse error.
                         * @member {org.dash.platform.dapi.v0.IStateTransitionBroadcastError|null|undefined} error
                         * @memberof org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse
                         * @instance
                         */
                        WaitForStateTransitionResultResponse.prototype.error = null;

                        /**
                         * WaitForStateTransitionResultResponse proof.
                         * @member {org.dash.platform.dapi.v0.IProvedResult|null|undefined} proof
                         * @memberof org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse
                         * @instance
                         */
                        WaitForStateTransitionResultResponse.prototype.proof = null;

                        /**
                         * WaitForStateTransitionResultResponse metadata.
                         * @member {org.dash.platform.dapi.v0.IResponseMetadata|null|undefined} metadata
                         * @memberof org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse
                         * @instance
                         */
                        WaitForStateTransitionResultResponse.prototype.metadata = null;

                        // OneOf field names bound to virtual getters and setters
                        var $oneOfFields;

                        /**
                         * WaitForStateTransitionResultResponse responses.
                         * @member {"error"|"proof"|undefined} responses
                         * @memberof org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse
                         * @instance
                         */
                        Object.defineProperty(WaitForStateTransitionResultResponse.prototype, "responses", {
                            get: $util.oneOfGetter($oneOfFields = ["error", "proof"]),
                            set: $util.oneOfSetter($oneOfFields)
                        });

                        /**
                         * Creates a new WaitForStateTransitionResultResponse instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.IWaitForStateTransitionResultResponse=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse} WaitForStateTransitionResultResponse instance
                         */
                        WaitForStateTransitionResultResponse.create = function create(properties) {
                            return new WaitForStateTransitionResultResponse(properties);
                        };

                        /**
                         * Encodes the specified WaitForStateTransitionResultResponse message. Does not implicitly {@link org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.IWaitForStateTransitionResultResponse} message WaitForStateTransitionResultResponse message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        WaitForStateTransitionResultResponse.encode = function encode(message, writer) {
                            if (!writer)
                                writer = $Writer.create();
                            if (message.error != null && Object.hasOwnProperty.call(message, "error"))
                                $root.org.dash.platform.dapi.v0.StateTransitionBroadcastError.encode(message.error, writer.uint32(/* id 1, wireType 2 =*/10).fork()).ldelim();
                            if (message.proof != null && Object.hasOwnProperty.call(message, "proof"))
                                $root.org.dash.platform.dapi.v0.ProvedResult.encode(message.proof, writer.uint32(/* id 2, wireType 2 =*/18).fork()).ldelim();
                            if (message.metadata != null && Object.hasOwnProperty.call(message, "metadata"))
                                $root.org.dash.platform.dapi.v0.ResponseMetadata.encode(message.metadata, writer.uint32(/* id 3, wireType 2 =*/26).fork()).ldelim();
                            return writer;
                        };

                        /**
                         * Encodes the specified WaitForStateTransitionResultResponse message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.IWaitForStateTransitionResultResponse} message WaitForStateTransitionResultResponse message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        WaitForStateTransitionResultResponse.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes a WaitForStateTransitionResultResponse message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse} WaitForStateTransitionResultResponse
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        WaitForStateTransitionResultResponse.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse();
                            while (reader.pos < end) {
                                var tag = reader.uint32();
                                switch (tag >>> 3) {
                                case 1:
                                    message.error = $root.org.dash.platform.dapi.v0.StateTransitionBroadcastError.decode(reader, reader.uint32());
                                    break;
                                case 2:
                                    message.proof = $root.org.dash.platform.dapi.v0.ProvedResult.decode(reader, reader.uint32());
                                    break;
                                case 3:
                                    message.metadata = $root.org.dash.platform.dapi.v0.ResponseMetadata.decode(reader, reader.uint32());
                                    break;
                                default:
                                    reader.skipType(tag & 7);
                                    break;
                                }
                            }
                            return message;
                        };

                        /**
                         * Decodes a WaitForStateTransitionResultResponse message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse} WaitForStateTransitionResultResponse
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        WaitForStateTransitionResultResponse.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies a WaitForStateTransitionResultResponse message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        WaitForStateTransitionResultResponse.verify = function verify(message) {
                            if (typeof message !== "object" || message === null)
                                return "object expected";
                            var properties = {};
                            if (message.error != null && message.hasOwnProperty("error")) {
                                properties.responses = 1;
                                {
                                    var error = $root.org.dash.platform.dapi.v0.StateTransitionBroadcastError.verify(message.error);
                                    if (error)
                                        return "error." + error;
                                }
                            }
                            if (message.proof != null && message.hasOwnProperty("proof")) {
                                if (properties.responses === 1)
                                    return "responses: multiple values";
                                properties.responses = 1;
                                {
                                    var error = $root.org.dash.platform.dapi.v0.ProvedResult.verify(message.proof);
                                    if (error)
                                        return "proof." + error;
                                }
                            }
                            if (message.metadata != null && message.hasOwnProperty("metadata")) {
                                var error = $root.org.dash.platform.dapi.v0.ResponseMetadata.verify(message.metadata);
                                if (error)
                                    return "metadata." + error;
                            }
                            return null;
                        };

                        /**
                         * Creates a WaitForStateTransitionResultResponse message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse} WaitForStateTransitionResultResponse
                         */
                        WaitForStateTransitionResultResponse.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse)
                                return object;
                            var message = new $root.org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse();
                            if (object.error != null) {
                                if (typeof object.error !== "object")
                                    throw TypeError(".org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.error: object expected");
                                message.error = $root.org.dash.platform.dapi.v0.StateTransitionBroadcastError.fromObject(object.error);
                            }
                            if (object.proof != null) {
                                if (typeof object.proof !== "object")
                                    throw TypeError(".org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.proof: object expected");
                                message.proof = $root.org.dash.platform.dapi.v0.ProvedResult.fromObject(object.proof);
                            }
                            if (object.metadata != null) {
                                if (typeof object.metadata !== "object")
                                    throw TypeError(".org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse.metadata: object expected");
                                message.metadata = $root.org.dash.platform.dapi.v0.ResponseMetadata.fromObject(object.metadata);
                            }
                            return message;
                        };

                        /**
                         * Creates a plain object from a WaitForStateTransitionResultResponse message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse} message WaitForStateTransitionResultResponse
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        WaitForStateTransitionResultResponse.toObject = function toObject(message, options) {
                            if (!options)
                                options = {};
                            var object = {};
                            if (options.defaults)
                                object.metadata = null;
                            if (message.error != null && message.hasOwnProperty("error")) {
                                object.error = $root.org.dash.platform.dapi.v0.StateTransitionBroadcastError.toObject(message.error, options);
                                if (options.oneofs)
                                    object.responses = "error";
                            }
                            if (message.proof != null && message.hasOwnProperty("proof")) {
                                object.proof = $root.org.dash.platform.dapi.v0.ProvedResult.toObject(message.proof, options);
                                if (options.oneofs)
                                    object.responses = "proof";
                            }
                            if (message.metadata != null && message.hasOwnProperty("metadata"))
                                object.metadata = $root.org.dash.platform.dapi.v0.ResponseMetadata.toObject(message.metadata, options);
                            return object;
                        };

                        /**
                         * Converts this WaitForStateTransitionResultResponse to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.WaitForStateTransitionResultResponse
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        WaitForStateTransitionResultResponse.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        return WaitForStateTransitionResultResponse;
                    })();

                    v0.ConsensusParamsBlock = (function() {

                        /**
                         * Properties of a ConsensusParamsBlock.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface IConsensusParamsBlock
                         * @property {string|null} [maxBytes] ConsensusParamsBlock maxBytes
                         * @property {string|null} [maxGas] ConsensusParamsBlock maxGas
                         * @property {string|null} [timeIotaMs] ConsensusParamsBlock timeIotaMs
                         */

                        /**
                         * Constructs a new ConsensusParamsBlock.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a ConsensusParamsBlock.
                         * @implements IConsensusParamsBlock
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.IConsensusParamsBlock=} [properties] Properties to set
                         */
                        function ConsensusParamsBlock(properties) {
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * ConsensusParamsBlock maxBytes.
                         * @member {string} maxBytes
                         * @memberof org.dash.platform.dapi.v0.ConsensusParamsBlock
                         * @instance
                         */
                        ConsensusParamsBlock.prototype.maxBytes = "";

                        /**
                         * ConsensusParamsBlock maxGas.
                         * @member {string} maxGas
                         * @memberof org.dash.platform.dapi.v0.ConsensusParamsBlock
                         * @instance
                         */
                        ConsensusParamsBlock.prototype.maxGas = "";

                        /**
                         * ConsensusParamsBlock timeIotaMs.
                         * @member {string} timeIotaMs
                         * @memberof org.dash.platform.dapi.v0.ConsensusParamsBlock
                         * @instance
                         */
                        ConsensusParamsBlock.prototype.timeIotaMs = "";

                        /**
                         * Creates a new ConsensusParamsBlock instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.ConsensusParamsBlock
                         * @static
                         * @param {org.dash.platform.dapi.v0.IConsensusParamsBlock=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.ConsensusParamsBlock} ConsensusParamsBlock instance
                         */
                        ConsensusParamsBlock.create = function create(properties) {
                            return new ConsensusParamsBlock(properties);
                        };

                        /**
                         * Encodes the specified ConsensusParamsBlock message. Does not implicitly {@link org.dash.platform.dapi.v0.ConsensusParamsBlock.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.ConsensusParamsBlock
                         * @static
                         * @param {org.dash.platform.dapi.v0.IConsensusParamsBlock} message ConsensusParamsBlock message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        ConsensusParamsBlock.encode = function encode(message, writer) {
                            if (!writer)
                                writer = $Writer.create();
                            if (message.maxBytes != null && Object.hasOwnProperty.call(message, "maxBytes"))
                                writer.uint32(/* id 1, wireType 2 =*/10).string(message.maxBytes);
                            if (message.maxGas != null && Object.hasOwnProperty.call(message, "maxGas"))
                                writer.uint32(/* id 2, wireType 2 =*/18).string(message.maxGas);
                            if (message.timeIotaMs != null && Object.hasOwnProperty.call(message, "timeIotaMs"))
                                writer.uint32(/* id 3, wireType 2 =*/26).string(message.timeIotaMs);
                            return writer;
                        };

                        /**
                         * Encodes the specified ConsensusParamsBlock message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.ConsensusParamsBlock.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.ConsensusParamsBlock
                         * @static
                         * @param {org.dash.platform.dapi.v0.IConsensusParamsBlock} message ConsensusParamsBlock message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        ConsensusParamsBlock.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes a ConsensusParamsBlock message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.ConsensusParamsBlock
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.ConsensusParamsBlock} ConsensusParamsBlock
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        ConsensusParamsBlock.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.ConsensusParamsBlock();
                            while (reader.pos < end) {
                                var tag = reader.uint32();
                                switch (tag >>> 3) {
                                case 1:
                                    message.maxBytes = reader.string();
                                    break;
                                case 2:
                                    message.maxGas = reader.string();
                                    break;
                                case 3:
                                    message.timeIotaMs = reader.string();
                                    break;
                                default:
                                    reader.skipType(tag & 7);
                                    break;
                                }
                            }
                            return message;
                        };

                        /**
                         * Decodes a ConsensusParamsBlock message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.ConsensusParamsBlock
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.ConsensusParamsBlock} ConsensusParamsBlock
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        ConsensusParamsBlock.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies a ConsensusParamsBlock message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.ConsensusParamsBlock
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        ConsensusParamsBlock.verify = function verify(message) {
                            if (typeof message !== "object" || message === null)
                                return "object expected";
                            if (message.maxBytes != null && message.hasOwnProperty("maxBytes"))
                                if (!$util.isString(message.maxBytes))
                                    return "maxBytes: string expected";
                            if (message.maxGas != null && message.hasOwnProperty("maxGas"))
                                if (!$util.isString(message.maxGas))
                                    return "maxGas: string expected";
                            if (message.timeIotaMs != null && message.hasOwnProperty("timeIotaMs"))
                                if (!$util.isString(message.timeIotaMs))
                                    return "timeIotaMs: string expected";
                            return null;
                        };

                        /**
                         * Creates a ConsensusParamsBlock message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.ConsensusParamsBlock
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.ConsensusParamsBlock} ConsensusParamsBlock
                         */
                        ConsensusParamsBlock.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.ConsensusParamsBlock)
                                return object;
                            var message = new $root.org.dash.platform.dapi.v0.ConsensusParamsBlock();
                            if (object.maxBytes != null)
                                message.maxBytes = String(object.maxBytes);
                            if (object.maxGas != null)
                                message.maxGas = String(object.maxGas);
                            if (object.timeIotaMs != null)
                                message.timeIotaMs = String(object.timeIotaMs);
                            return message;
                        };

                        /**
                         * Creates a plain object from a ConsensusParamsBlock message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.ConsensusParamsBlock
                         * @static
                         * @param {org.dash.platform.dapi.v0.ConsensusParamsBlock} message ConsensusParamsBlock
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        ConsensusParamsBlock.toObject = function toObject(message, options) {
                            if (!options)
                                options = {};
                            var object = {};
                            if (options.defaults) {
                                object.maxBytes = "";
                                object.maxGas = "";
                                object.timeIotaMs = "";
                            }
                            if (message.maxBytes != null && message.hasOwnProperty("maxBytes"))
                                object.maxBytes = message.maxBytes;
                            if (message.maxGas != null && message.hasOwnProperty("maxGas"))
                                object.maxGas = message.maxGas;
                            if (message.timeIotaMs != null && message.hasOwnProperty("timeIotaMs"))
                                object.timeIotaMs = message.timeIotaMs;
                            return object;
                        };

                        /**
                         * Converts this ConsensusParamsBlock to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.ConsensusParamsBlock
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        ConsensusParamsBlock.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        return ConsensusParamsBlock;
                    })();

                    v0.ConsensusParamsEvidence = (function() {

                        /**
                         * Properties of a ConsensusParamsEvidence.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface IConsensusParamsEvidence
                         * @property {string|null} [maxAgeNumBlocks] ConsensusParamsEvidence maxAgeNumBlocks
                         * @property {string|null} [maxAgeDuration] ConsensusParamsEvidence maxAgeDuration
                         * @property {string|null} [maxBytes] ConsensusParamsEvidence maxBytes
                         */

                        /**
                         * Constructs a new ConsensusParamsEvidence.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a ConsensusParamsEvidence.
                         * @implements IConsensusParamsEvidence
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.IConsensusParamsEvidence=} [properties] Properties to set
                         */
                        function ConsensusParamsEvidence(properties) {
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * ConsensusParamsEvidence maxAgeNumBlocks.
                         * @member {string} maxAgeNumBlocks
                         * @memberof org.dash.platform.dapi.v0.ConsensusParamsEvidence
                         * @instance
                         */
                        ConsensusParamsEvidence.prototype.maxAgeNumBlocks = "";

                        /**
                         * ConsensusParamsEvidence maxAgeDuration.
                         * @member {string} maxAgeDuration
                         * @memberof org.dash.platform.dapi.v0.ConsensusParamsEvidence
                         * @instance
                         */
                        ConsensusParamsEvidence.prototype.maxAgeDuration = "";

                        /**
                         * ConsensusParamsEvidence maxBytes.
                         * @member {string} maxBytes
                         * @memberof org.dash.platform.dapi.v0.ConsensusParamsEvidence
                         * @instance
                         */
                        ConsensusParamsEvidence.prototype.maxBytes = "";

                        /**
                         * Creates a new ConsensusParamsEvidence instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.ConsensusParamsEvidence
                         * @static
                         * @param {org.dash.platform.dapi.v0.IConsensusParamsEvidence=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.ConsensusParamsEvidence} ConsensusParamsEvidence instance
                         */
                        ConsensusParamsEvidence.create = function create(properties) {
                            return new ConsensusParamsEvidence(properties);
                        };

                        /**
                         * Encodes the specified ConsensusParamsEvidence message. Does not implicitly {@link org.dash.platform.dapi.v0.ConsensusParamsEvidence.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.ConsensusParamsEvidence
                         * @static
                         * @param {org.dash.platform.dapi.v0.IConsensusParamsEvidence} message ConsensusParamsEvidence message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        ConsensusParamsEvidence.encode = function encode(message, writer) {
                            if (!writer)
                                writer = $Writer.create();
                            if (message.maxAgeNumBlocks != null && Object.hasOwnProperty.call(message, "maxAgeNumBlocks"))
                                writer.uint32(/* id 1, wireType 2 =*/10).string(message.maxAgeNumBlocks);
                            if (message.maxAgeDuration != null && Object.hasOwnProperty.call(message, "maxAgeDuration"))
                                writer.uint32(/* id 2, wireType 2 =*/18).string(message.maxAgeDuration);
                            if (message.maxBytes != null && Object.hasOwnProperty.call(message, "maxBytes"))
                                writer.uint32(/* id 3, wireType 2 =*/26).string(message.maxBytes);
                            return writer;
                        };

                        /**
                         * Encodes the specified ConsensusParamsEvidence message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.ConsensusParamsEvidence.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.ConsensusParamsEvidence
                         * @static
                         * @param {org.dash.platform.dapi.v0.IConsensusParamsEvidence} message ConsensusParamsEvidence message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        ConsensusParamsEvidence.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes a ConsensusParamsEvidence message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.ConsensusParamsEvidence
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.ConsensusParamsEvidence} ConsensusParamsEvidence
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        ConsensusParamsEvidence.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.ConsensusParamsEvidence();
                            while (reader.pos < end) {
                                var tag = reader.uint32();
                                switch (tag >>> 3) {
                                case 1:
                                    message.maxAgeNumBlocks = reader.string();
                                    break;
                                case 2:
                                    message.maxAgeDuration = reader.string();
                                    break;
                                case 3:
                                    message.maxBytes = reader.string();
                                    break;
                                default:
                                    reader.skipType(tag & 7);
                                    break;
                                }
                            }
                            return message;
                        };

                        /**
                         * Decodes a ConsensusParamsEvidence message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.ConsensusParamsEvidence
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.ConsensusParamsEvidence} ConsensusParamsEvidence
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        ConsensusParamsEvidence.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies a ConsensusParamsEvidence message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.ConsensusParamsEvidence
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        ConsensusParamsEvidence.verify = function verify(message) {
                            if (typeof message !== "object" || message === null)
                                return "object expected";
                            if (message.maxAgeNumBlocks != null && message.hasOwnProperty("maxAgeNumBlocks"))
                                if (!$util.isString(message.maxAgeNumBlocks))
                                    return "maxAgeNumBlocks: string expected";
                            if (message.maxAgeDuration != null && message.hasOwnProperty("maxAgeDuration"))
                                if (!$util.isString(message.maxAgeDuration))
                                    return "maxAgeDuration: string expected";
                            if (message.maxBytes != null && message.hasOwnProperty("maxBytes"))
                                if (!$util.isString(message.maxBytes))
                                    return "maxBytes: string expected";
                            return null;
                        };

                        /**
                         * Creates a ConsensusParamsEvidence message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.ConsensusParamsEvidence
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.ConsensusParamsEvidence} ConsensusParamsEvidence
                         */
                        ConsensusParamsEvidence.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.ConsensusParamsEvidence)
                                return object;
                            var message = new $root.org.dash.platform.dapi.v0.ConsensusParamsEvidence();
                            if (object.maxAgeNumBlocks != null)
                                message.maxAgeNumBlocks = String(object.maxAgeNumBlocks);
                            if (object.maxAgeDuration != null)
                                message.maxAgeDuration = String(object.maxAgeDuration);
                            if (object.maxBytes != null)
                                message.maxBytes = String(object.maxBytes);
                            return message;
                        };

                        /**
                         * Creates a plain object from a ConsensusParamsEvidence message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.ConsensusParamsEvidence
                         * @static
                         * @param {org.dash.platform.dapi.v0.ConsensusParamsEvidence} message ConsensusParamsEvidence
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        ConsensusParamsEvidence.toObject = function toObject(message, options) {
                            if (!options)
                                options = {};
                            var object = {};
                            if (options.defaults) {
                                object.maxAgeNumBlocks = "";
                                object.maxAgeDuration = "";
                                object.maxBytes = "";
                            }
                            if (message.maxAgeNumBlocks != null && message.hasOwnProperty("maxAgeNumBlocks"))
                                object.maxAgeNumBlocks = message.maxAgeNumBlocks;
                            if (message.maxAgeDuration != null && message.hasOwnProperty("maxAgeDuration"))
                                object.maxAgeDuration = message.maxAgeDuration;
                            if (message.maxBytes != null && message.hasOwnProperty("maxBytes"))
                                object.maxBytes = message.maxBytes;
                            return object;
                        };

                        /**
                         * Converts this ConsensusParamsEvidence to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.ConsensusParamsEvidence
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        ConsensusParamsEvidence.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        return ConsensusParamsEvidence;
                    })();

                    v0.GetConsensusParamsRequest = (function() {

                        /**
                         * Properties of a GetConsensusParamsRequest.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface IGetConsensusParamsRequest
                         * @property {number|Long|null} [height] GetConsensusParamsRequest height
                         * @property {boolean|null} [prove] GetConsensusParamsRequest prove
                         */

                        /**
                         * Constructs a new GetConsensusParamsRequest.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a GetConsensusParamsRequest.
                         * @implements IGetConsensusParamsRequest
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.IGetConsensusParamsRequest=} [properties] Properties to set
                         */
                        function GetConsensusParamsRequest(properties) {
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * GetConsensusParamsRequest height.
                         * @member {number|Long} height
                         * @memberof org.dash.platform.dapi.v0.GetConsensusParamsRequest
                         * @instance
                         */
                        GetConsensusParamsRequest.prototype.height = $util.Long ? $util.Long.fromBits(0,0,false) : 0;

                        /**
                         * GetConsensusParamsRequest prove.
                         * @member {boolean} prove
                         * @memberof org.dash.platform.dapi.v0.GetConsensusParamsRequest
                         * @instance
                         */
                        GetConsensusParamsRequest.prototype.prove = false;

                        /**
                         * Creates a new GetConsensusParamsRequest instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.GetConsensusParamsRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetConsensusParamsRequest=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.GetConsensusParamsRequest} GetConsensusParamsRequest instance
                         */
                        GetConsensusParamsRequest.create = function create(properties) {
                            return new GetConsensusParamsRequest(properties);
                        };

                        /**
                         * Encodes the specified GetConsensusParamsRequest message. Does not implicitly {@link org.dash.platform.dapi.v0.GetConsensusParamsRequest.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.GetConsensusParamsRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetConsensusParamsRequest} message GetConsensusParamsRequest message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        GetConsensusParamsRequest.encode = function encode(message, writer) {
                            if (!writer)
                                writer = $Writer.create();
                            if (message.height != null && Object.hasOwnProperty.call(message, "height"))
                                writer.uint32(/* id 1, wireType 0 =*/8).int64(message.height);
                            if (message.prove != null && Object.hasOwnProperty.call(message, "prove"))
                                writer.uint32(/* id 2, wireType 0 =*/16).bool(message.prove);
                            return writer;
                        };

                        /**
                         * Encodes the specified GetConsensusParamsRequest message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.GetConsensusParamsRequest.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.GetConsensusParamsRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetConsensusParamsRequest} message GetConsensusParamsRequest message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        GetConsensusParamsRequest.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes a GetConsensusParamsRequest message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.GetConsensusParamsRequest
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.GetConsensusParamsRequest} GetConsensusParamsRequest
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        GetConsensusParamsRequest.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.GetConsensusParamsRequest();
                            while (reader.pos < end) {
                                var tag = reader.uint32();
                                switch (tag >>> 3) {
                                case 1:
                                    message.height = reader.int64();
                                    break;
                                case 2:
                                    message.prove = reader.bool();
                                    break;
                                default:
                                    reader.skipType(tag & 7);
                                    break;
                                }
                            }
                            return message;
                        };

                        /**
                         * Decodes a GetConsensusParamsRequest message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.GetConsensusParamsRequest
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.GetConsensusParamsRequest} GetConsensusParamsRequest
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        GetConsensusParamsRequest.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies a GetConsensusParamsRequest message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.GetConsensusParamsRequest
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        GetConsensusParamsRequest.verify = function verify(message) {
                            if (typeof message !== "object" || message === null)
                                return "object expected";
                            if (message.height != null && message.hasOwnProperty("height"))
                                if (!$util.isInteger(message.height) && !(message.height && $util.isInteger(message.height.low) && $util.isInteger(message.height.high)))
                                    return "height: integer|Long expected";
                            if (message.prove != null && message.hasOwnProperty("prove"))
                                if (typeof message.prove !== "boolean")
                                    return "prove: boolean expected";
                            return null;
                        };

                        /**
                         * Creates a GetConsensusParamsRequest message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.GetConsensusParamsRequest
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.GetConsensusParamsRequest} GetConsensusParamsRequest
                         */
                        GetConsensusParamsRequest.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.GetConsensusParamsRequest)
                                return object;
                            var message = new $root.org.dash.platform.dapi.v0.GetConsensusParamsRequest();
                            if (object.height != null)
                                if ($util.Long)
                                    (message.height = $util.Long.fromValue(object.height)).unsigned = false;
                                else if (typeof object.height === "string")
                                    message.height = parseInt(object.height, 10);
                                else if (typeof object.height === "number")
                                    message.height = object.height;
                                else if (typeof object.height === "object")
                                    message.height = new $util.LongBits(object.height.low >>> 0, object.height.high >>> 0).toNumber();
                            if (object.prove != null)
                                message.prove = Boolean(object.prove);
                            return message;
                        };

                        /**
                         * Creates a plain object from a GetConsensusParamsRequest message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.GetConsensusParamsRequest
                         * @static
                         * @param {org.dash.platform.dapi.v0.GetConsensusParamsRequest} message GetConsensusParamsRequest
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        GetConsensusParamsRequest.toObject = function toObject(message, options) {
                            if (!options)
                                options = {};
                            var object = {};
                            if (options.defaults) {
                                if ($util.Long) {
                                    var long = new $util.Long(0, 0, false);
                                    object.height = options.longs === String ? long.toString() : options.longs === Number ? long.toNumber() : long;
                                } else
                                    object.height = options.longs === String ? "0" : 0;
                                object.prove = false;
                            }
                            if (message.height != null && message.hasOwnProperty("height"))
                                if (typeof message.height === "number")
                                    object.height = options.longs === String ? String(message.height) : message.height;
                                else
                                    object.height = options.longs === String ? $util.Long.prototype.toString.call(message.height) : options.longs === Number ? new $util.LongBits(message.height.low >>> 0, message.height.high >>> 0).toNumber() : message.height;
                            if (message.prove != null && message.hasOwnProperty("prove"))
                                object.prove = message.prove;
                            return object;
                        };

                        /**
                         * Converts this GetConsensusParamsRequest to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.GetConsensusParamsRequest
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        GetConsensusParamsRequest.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        return GetConsensusParamsRequest;
                    })();

                    v0.GetConsensusParamsResponse = (function() {

                        /**
                         * Properties of a GetConsensusParamsResponse.
                         * @memberof org.dash.platform.dapi.v0
                         * @interface IGetConsensusParamsResponse
                         * @property {org.dash.platform.dapi.v0.IConsensusParamsBlock|null} [block] GetConsensusParamsResponse block
                         * @property {org.dash.platform.dapi.v0.IConsensusParamsEvidence|null} [evidence] GetConsensusParamsResponse evidence
                         */

                        /**
                         * Constructs a new GetConsensusParamsResponse.
                         * @memberof org.dash.platform.dapi.v0
                         * @classdesc Represents a GetConsensusParamsResponse.
                         * @implements IGetConsensusParamsResponse
                         * @constructor
                         * @param {org.dash.platform.dapi.v0.IGetConsensusParamsResponse=} [properties] Properties to set
                         */
                        function GetConsensusParamsResponse(properties) {
                            if (properties)
                                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                                    if (properties[keys[i]] != null)
                                        this[keys[i]] = properties[keys[i]];
                        }

                        /**
                         * GetConsensusParamsResponse block.
                         * @member {org.dash.platform.dapi.v0.IConsensusParamsBlock|null|undefined} block
                         * @memberof org.dash.platform.dapi.v0.GetConsensusParamsResponse
                         * @instance
                         */
                        GetConsensusParamsResponse.prototype.block = null;

                        /**
                         * GetConsensusParamsResponse evidence.
                         * @member {org.dash.platform.dapi.v0.IConsensusParamsEvidence|null|undefined} evidence
                         * @memberof org.dash.platform.dapi.v0.GetConsensusParamsResponse
                         * @instance
                         */
                        GetConsensusParamsResponse.prototype.evidence = null;

                        /**
                         * Creates a new GetConsensusParamsResponse instance using the specified properties.
                         * @function create
                         * @memberof org.dash.platform.dapi.v0.GetConsensusParamsResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetConsensusParamsResponse=} [properties] Properties to set
                         * @returns {org.dash.platform.dapi.v0.GetConsensusParamsResponse} GetConsensusParamsResponse instance
                         */
                        GetConsensusParamsResponse.create = function create(properties) {
                            return new GetConsensusParamsResponse(properties);
                        };

                        /**
                         * Encodes the specified GetConsensusParamsResponse message. Does not implicitly {@link org.dash.platform.dapi.v0.GetConsensusParamsResponse.verify|verify} messages.
                         * @function encode
                         * @memberof org.dash.platform.dapi.v0.GetConsensusParamsResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetConsensusParamsResponse} message GetConsensusParamsResponse message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        GetConsensusParamsResponse.encode = function encode(message, writer) {
                            if (!writer)
                                writer = $Writer.create();
                            if (message.block != null && Object.hasOwnProperty.call(message, "block"))
                                $root.org.dash.platform.dapi.v0.ConsensusParamsBlock.encode(message.block, writer.uint32(/* id 1, wireType 2 =*/10).fork()).ldelim();
                            if (message.evidence != null && Object.hasOwnProperty.call(message, "evidence"))
                                $root.org.dash.platform.dapi.v0.ConsensusParamsEvidence.encode(message.evidence, writer.uint32(/* id 2, wireType 2 =*/18).fork()).ldelim();
                            return writer;
                        };

                        /**
                         * Encodes the specified GetConsensusParamsResponse message, length delimited. Does not implicitly {@link org.dash.platform.dapi.v0.GetConsensusParamsResponse.verify|verify} messages.
                         * @function encodeDelimited
                         * @memberof org.dash.platform.dapi.v0.GetConsensusParamsResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.IGetConsensusParamsResponse} message GetConsensusParamsResponse message or plain object to encode
                         * @param {$protobuf.Writer} [writer] Writer to encode to
                         * @returns {$protobuf.Writer} Writer
                         */
                        GetConsensusParamsResponse.encodeDelimited = function encodeDelimited(message, writer) {
                            return this.encode(message, writer).ldelim();
                        };

                        /**
                         * Decodes a GetConsensusParamsResponse message from the specified reader or buffer.
                         * @function decode
                         * @memberof org.dash.platform.dapi.v0.GetConsensusParamsResponse
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @param {number} [length] Message length if known beforehand
                         * @returns {org.dash.platform.dapi.v0.GetConsensusParamsResponse} GetConsensusParamsResponse
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        GetConsensusParamsResponse.decode = function decode(reader, length) {
                            if (!(reader instanceof $Reader))
                                reader = $Reader.create(reader);
                            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.org.dash.platform.dapi.v0.GetConsensusParamsResponse();
                            while (reader.pos < end) {
                                var tag = reader.uint32();
                                switch (tag >>> 3) {
                                case 1:
                                    message.block = $root.org.dash.platform.dapi.v0.ConsensusParamsBlock.decode(reader, reader.uint32());
                                    break;
                                case 2:
                                    message.evidence = $root.org.dash.platform.dapi.v0.ConsensusParamsEvidence.decode(reader, reader.uint32());
                                    break;
                                default:
                                    reader.skipType(tag & 7);
                                    break;
                                }
                            }
                            return message;
                        };

                        /**
                         * Decodes a GetConsensusParamsResponse message from the specified reader or buffer, length delimited.
                         * @function decodeDelimited
                         * @memberof org.dash.platform.dapi.v0.GetConsensusParamsResponse
                         * @static
                         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
                         * @returns {org.dash.platform.dapi.v0.GetConsensusParamsResponse} GetConsensusParamsResponse
                         * @throws {Error} If the payload is not a reader or valid buffer
                         * @throws {$protobuf.util.ProtocolError} If required fields are missing
                         */
                        GetConsensusParamsResponse.decodeDelimited = function decodeDelimited(reader) {
                            if (!(reader instanceof $Reader))
                                reader = new $Reader(reader);
                            return this.decode(reader, reader.uint32());
                        };

                        /**
                         * Verifies a GetConsensusParamsResponse message.
                         * @function verify
                         * @memberof org.dash.platform.dapi.v0.GetConsensusParamsResponse
                         * @static
                         * @param {Object.<string,*>} message Plain object to verify
                         * @returns {string|null} `null` if valid, otherwise the reason why it is not
                         */
                        GetConsensusParamsResponse.verify = function verify(message) {
                            if (typeof message !== "object" || message === null)
                                return "object expected";
                            if (message.block != null && message.hasOwnProperty("block")) {
                                var error = $root.org.dash.platform.dapi.v0.ConsensusParamsBlock.verify(message.block);
                                if (error)
                                    return "block." + error;
                            }
                            if (message.evidence != null && message.hasOwnProperty("evidence")) {
                                var error = $root.org.dash.platform.dapi.v0.ConsensusParamsEvidence.verify(message.evidence);
                                if (error)
                                    return "evidence." + error;
                            }
                            return null;
                        };

                        /**
                         * Creates a GetConsensusParamsResponse message from a plain object. Also converts values to their respective internal types.
                         * @function fromObject
                         * @memberof org.dash.platform.dapi.v0.GetConsensusParamsResponse
                         * @static
                         * @param {Object.<string,*>} object Plain object
                         * @returns {org.dash.platform.dapi.v0.GetConsensusParamsResponse} GetConsensusParamsResponse
                         */
                        GetConsensusParamsResponse.fromObject = function fromObject(object) {
                            if (object instanceof $root.org.dash.platform.dapi.v0.GetConsensusParamsResponse)
                                return object;
                            var message = new $root.org.dash.platform.dapi.v0.GetConsensusParamsResponse();
                            if (object.block != null) {
                                if (typeof object.block !== "object")
                                    throw TypeError(".org.dash.platform.dapi.v0.GetConsensusParamsResponse.block: object expected");
                                message.block = $root.org.dash.platform.dapi.v0.ConsensusParamsBlock.fromObject(object.block);
                            }
                            if (object.evidence != null) {
                                if (typeof object.evidence !== "object")
                                    throw TypeError(".org.dash.platform.dapi.v0.GetConsensusParamsResponse.evidence: object expected");
                                message.evidence = $root.org.dash.platform.dapi.v0.ConsensusParamsEvidence.fromObject(object.evidence);
                            }
                            return message;
                        };

                        /**
                         * Creates a plain object from a GetConsensusParamsResponse message. Also converts values to other types if specified.
                         * @function toObject
                         * @memberof org.dash.platform.dapi.v0.GetConsensusParamsResponse
                         * @static
                         * @param {org.dash.platform.dapi.v0.GetConsensusParamsResponse} message GetConsensusParamsResponse
                         * @param {$protobuf.IConversionOptions} [options] Conversion options
                         * @returns {Object.<string,*>} Plain object
                         */
                        GetConsensusParamsResponse.toObject = function toObject(message, options) {
                            if (!options)
                                options = {};
                            var object = {};
                            if (options.defaults) {
                                object.block = null;
                                object.evidence = null;
                            }
                            if (message.block != null && message.hasOwnProperty("block"))
                                object.block = $root.org.dash.platform.dapi.v0.ConsensusParamsBlock.toObject(message.block, options);
                            if (message.evidence != null && message.hasOwnProperty("evidence"))
                                object.evidence = $root.org.dash.platform.dapi.v0.ConsensusParamsEvidence.toObject(message.evidence, options);
                            return object;
                        };

                        /**
                         * Converts this GetConsensusParamsResponse to JSON.
                         * @function toJSON
                         * @memberof org.dash.platform.dapi.v0.GetConsensusParamsResponse
                         * @instance
                         * @returns {Object.<string,*>} JSON object
                         */
                        GetConsensusParamsResponse.prototype.toJSON = function toJSON() {
                            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
                        };

                        return GetConsensusParamsResponse;
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
